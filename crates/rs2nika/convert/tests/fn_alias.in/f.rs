fn notes_of(chain: &[String]) -> Vec<String> {
    let path = nikaia_std::tools::modules::pause_path;
    let mut notes = Vec::new();
    notes.push(format!("{}, which pauses here.", path(chain)));
    notes
}

fn shown(chain: &[String], unit: &Unit) -> String {
    let path = nikaia_std::tools::modules::pause_path;
    format!("{} at {}", path(chain), unit.path())
}
