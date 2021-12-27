pub trait Named {
    fn get_name(&self) -> &String;
    fn get_aliases(&self) -> &[String];

    fn matches(&self, str: &str) -> bool {
        self.get_name().eq_ignore_ascii_case(str)
            || self.get_aliases().iter().any(|alias| alias.eq_ignore_ascii_case(str))
    }
}
