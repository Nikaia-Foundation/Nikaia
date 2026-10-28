struct Own {
    root: PathBuf,
    variant: Option<String>,
}

impl Own {
    fn tag(&self) -> String {
        let mut named = self.root.to_string_lossy().into_owned();
        if let Some(variant) = &self.variant {
            named.push_str(variant);
        }
        named
    }

    fn copy(&self, built: &Path) -> Result<PathBuf> {
        let dir = self.root.join("bin");
        std::fs::create_dir_all(&dir)?;
        let name = built.file_name().context("a program has a file name")?;
        let own = dir.join(name);
        let _ = std::fs::remove_file(&own);
        if own.exists() {
            bail!("still there: {}", own.display());
        }
        Ok(own)
    }
}
