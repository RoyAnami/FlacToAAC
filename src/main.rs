use std::process::Command;
use std::path::Path;

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
