fn loaded_text(path: &Path) -> Result<String> {
    let text = load(path).context("cannot load")?;
    if text.is_empty() {
        return Err(crate::diagnostics::refuse(format!("{} is empty", path.display())));
    }
    Ok(text)
}

fn short_said(why: String) -> Result<()> {
    const MOST: usize = 3;
    if why.len() > MOST {
        return Err(anyhow!(why));
    }
    bail!("too short: {}", why);
}
