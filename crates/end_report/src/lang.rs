use clap::ValueEnum;

/// Report output language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lower")]
pub enum Lang {
    Zh,
    En,
}
