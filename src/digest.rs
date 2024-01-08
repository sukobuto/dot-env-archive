use std::path::Path;
use tokio::io::AsyncReadExt;

pub async fn file_checksum(file_path: &Path) -> anyhow::Result<String> {
    let mut context = ring::digest::Context::new(&ring::digest::SHA256);
    let file = tokio::fs::File::open(file_path).await?;
    let mut reader = tokio::io::BufReader::new(file);
    let mut buffer = [0; 1024];
    loop {
        let n = reader.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        context.update(&buffer[..n]);
    }
    let digest = context.finish();
    Ok(hex::encode(digest.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_file_checksum() {
        let file_path = Path::new("LICENSE");
        let checksum = file_checksum(file_path).await.unwrap();
        assert_eq!(
            checksum,
            "572f866d5425aa9ce56b042726c11a3ebad73922b78d4ad536d26fa91de67e49"
        );
    }
}
