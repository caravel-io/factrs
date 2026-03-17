use anyhow::Result;
use bosun;

fn main() -> Result<()> {
    factrs::build::run()?;
    Ok(())
}
