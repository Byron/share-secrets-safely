#[cfg(feature = "rest")]
pub fn output_formats() -> &'static [&'static str] {
    &["json", "yaml"]
}
