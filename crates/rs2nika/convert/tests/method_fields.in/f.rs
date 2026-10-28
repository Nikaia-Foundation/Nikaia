const STD_DIR: &str = "std";

pub struct Package {
    files: Vec<(String, String)>,
}

pub struct Cache {
    root: std::path::PathBuf,
    level: u32,
}

impl Cache {
    fn std_dir(&self) -> std::path::PathBuf {
        self.root.join(STD_DIR)
    }

    /// The cache's files, packed.
    fn pack(&self, name: &str) -> String {
        let dir = self.std_dir();
        let n = Self::count(self.level);
        format!("{}/{name}/{n}", dir.display())
    }

    fn count(level: u32) -> u32 {
        level + 1
    }
}

impl Package {
    fn unpack(text: &str) -> Option<Package> {
        let files = vec![(text.to_string(), String::new())];
        Some(Package { files })
    }
}
