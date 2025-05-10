use std::path::{Path, PathBuf};
use std::process::Command;
use rfd::FileDialog;

fn split_flac_with_cue(cue_path: &Path, flac_path: &Path) -> std::io::Result<()> {
    Command::new("flac-tracksplit")
        .arg(flac_path)
        .arg(cue_path)
        .status()?;
    Ok(())
}

fn convert_flac_to_aac(flac_path: &Path, aac_path: &Path) -> std::io::Result<()> {
    Command::new("qaac")
        .arg("--no-optimize")
        .arg("-V")
        .arg("127")
        .arg("-o")
        .arg(aac_path)
        .arg(flac_path)
        .status()?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    // ファイル選択ダイアログを表示して CUE ファイルを選ばせる
    let cue_file: Option<PathBuf> = FileDialog::new()
        .add_filter("CUE sheet", &["cue"])
        .set_title("CUEファイルを選択してください")
        .pick_file();

    let cue_file = match cue_file {
        Some(path) => path,
        None => {
            eprintln!("CUEファイルが選択されませんでした。");
            return Ok(());
        }
    };

    // FLACファイルはCUEと同じ名前・ディレクトリにある想定
    let flac_file = cue_file.with_extension("flac");

    if !flac_file.exists() {
        eprintln!("対応するFLACファイルが見つかりません: {:?}", flac_file);
        return Ok(());
    }

    println!("CUE:  {:?}", cue_file);
    println!("FLAC: {:?}", flac_file);

    // 必要な処理をここから呼び出す
    split_flac_with_cue(&cue_file, &flac_file)?;

    // 以降、変換処理などへ続く

    Ok(())
}
