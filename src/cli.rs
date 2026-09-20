#[derive(Debug, Clone)]
pub struct CliArgs {
    pub link: String,
    pub context: String,
    pub vault: Option<String>,
    pub format: String,
    pub verbose: usize,
    pub emplacement: bool,
}
