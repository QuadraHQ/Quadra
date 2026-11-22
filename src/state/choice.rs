pub trait Choice: Copy + PartialEq + 'static {
    const ALL: &'static [Self];

    fn label(self) -> &'static str;
}
