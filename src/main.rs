use rfd::FileDialog;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::fs;
use std::process::Command;
use encoding_rs::SHIFT_JIS;
use std::env;

fn sanitize_filename(name: &str) -> String {
    name.replace(&['/', '\\', ':', '*', '?', '"', '<', '>', '|'][..], "_")
}

#[derive(Debug)]
struct CueTrackInfo {
    title: Option<String>,
    performer: Option<String>,
}

fn parse_cue(
    cue_path: &str,
) -> (
    HashMap<u32, CueTrackInfo>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let raw_bytes = fs::read(cue_path).expect("CUEファイルの読み込みに失敗しました");
    let (decoded, _, _) = SHIFT_JIS.decode(&raw_bytes);
    let content = decoded;

    let mut tracks = HashMap::new();
    let mut current_track: Option<u32> = None;
    let mut global_title = None;
    let mut global_performer = None;
    let mut global_date = None;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("TRACK") {
            if let Some(track_num) = trimmed
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse::<u32>().ok())
            {
                current_track = Some(track_num);
                tracks.entry(track_num).or_insert(CueTrackInfo {
                    title: None,
                    performer: None,
                });
            }
        } else if trimmed.starts_with("TITLE") {
            let title = trimmed.trim_start_matches("TITLE").trim().trim_matches('"').to_string();
            if let Some(track_num) = current_track {
                tracks.entry(track_num).and_modify(|info| info.title = Some(title.clone()));
            } else {
                global_title = Some(title);
            }
        } else if trimmed.starts_with("PERFORMER") {
            let performer = trimmed.trim_start_matches("PERFORMER").trim().trim_matches('"').to_string();
            if let Some(track_num) = current_track {
                tracks.entry(track_num).and_modify(|info| info.performer = Some(performer.clone()));
            } else {
                global_performer = Some(performer);
            }
        } else if trimmed.starts_with("REM DATE") {
            let date = trimmed.trim_start_matches("REM DATE").trim().to_string();
            global_date = Some(date);
        }
    }

    (tracks, global_title, global_performer, global_date)
}

fn main() {
    // ダイアログでCUEファイルを選択
    let cue_path_buf: Option<PathBuf> = FileDialog::new()
        .add_filter("CUE sheet", &["cue"])
        .set_title("CUEファイルを選択してください")
        .pick_file();

    let cue_path_buf = match cue_path_buf {
        Some(path) => path,
        None => {
            eprintln!("キャンセルされました。");
            return;
        }
    };

    let cue_path = cue_path_buf.to_str().unwrap();  // cue_path_bufを文字列として取得
    let base_stem = cue_path_buf.file_stem().unwrap().to_string_lossy();
    let parent_dir = cue_path_buf.parent().unwrap();

    let flac_path = parent_dir.join(format!("{}.flac", base_stem));
    let artwork_path = parent_dir.join(format!("{}.jpg", base_stem));
    let png_path = parent_dir.join(format!("{}.png", base_stem));

    if !flac_path.exists() || !artwork_path.exists() {
        eprintln!("FLACまたはJPGが見つかりませんでした。");
        return;
    }

    // CUEから情報取得
    let (tracks, album_title, global_performer, global_date) = parse_cue(cue_path);

    let original_dir = env::current_dir().expect("カレントディレクトリの取得に失敗しました");
    env::set_current_dir(parent_dir).expect("ディレクトリの変更に失敗しました");

    // FLACを分割
    let split_status = Command::new("shntool")
        .args(&["split", "-f", cue_path, "-o", "flac", flac_path.to_str().unwrap()])
        .status()
        .expect("shntool の実行に失敗しました");

    // 元のカレントディレクトリに戻す
    env::set_current_dir(original_dir).expect("元のディレクトリに戻すのに失敗しました");

    if !split_status.success() {
        eprintln!("FLACの分割に失敗しました");
        return;
    }

    // 各トラックの変換
    for i in 1..=99 {
        let input_name = format!("split-track{:02}.flac", i);
        let input_path = parent_dir.join(&input_name);
        if !input_path.exists() {
            break;
        }

        let track_info = tracks.get(&(i as u32));

        let title = track_info
            .and_then(|info| info.title.as_ref())
            .map(|s| s.clone())
            .unwrap_or_else(|| "unknown".to_string());

        let performer = track_info
            .and_then(|info| info.performer.as_ref())
            .or(global_performer.as_ref())
            .map(|s| s.clone())
            .unwrap_or_else(|| "unknown".to_string());

        let album = album_title.clone().unwrap_or_else(|| "unknown album".to_string());
        let date = global_date.clone().unwrap_or_else(|| "unknown".to_string());

        let file_title = sanitize_filename(&title);
        let output_name = format!("{:02}_{}.m4a", i, file_title);
        let output_path = parent_dir.join(&output_name);

        let qaac_status = Command::new("qaac64")
            .args(&[
                "--no-optimize",
                "--tvbr", "91",
                "--verbose",
                "--artwork", artwork_path.to_str().unwrap(),
                "--title", &title,
                "--artist", &performer,
                "--album", &album,
                "--date", &date,
                "--track", &i.to_string(),
                input_path.to_str().unwrap(),
                "-o",
                output_path.to_str().unwrap(),
            ])
            .status()
            .expect("failed to run qaac");

        if qaac_status.success() {
            println!("変換成功: {}", output_name);
        } else {
            eprintln!("変換失敗: {}", input_name);
        }
    }

    // 元のFLAC, CUE, JPG、PNGと分割したFLACファイルを削除
    let files_to_delete = vec![
        flac_path,
        cue_path_buf.clone(),  // 修正：所有権を移動せずにクローンを使う
        artwork_path,
        png_path,
    ];

    // まず、元のファイルと分割したFLACファイルを削除
    for file in files_to_delete {
        if file.exists() {
            if let Err(e) = fs::remove_file(&file) {
                eprintln!("削除失敗: {}", e);
            } else {
                println!("削除成功: {}", file.display());
            }
        }
    }

    // 分割したFLACファイル群を削除
    for i in 1..=99 {
        let split_flac_path = parent_dir.join(format!("split-track{:02}.flac", i));
        if split_flac_path.exists() {
            if let Err(e) = fs::remove_file(&split_flac_path) {
                eprintln!("削除失敗: {}", e);
            } else {
                println!("削除成功: {}", split_flac_path.display());
            }
        }
    }
}
