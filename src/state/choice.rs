pub trait Choice: Copy + PartialEq + 'static {
    const ALL: &'static [Self];

    fn label(self) -> &'static str;

    fn labels() -> Vec<&'static str> {
        Self::ALL.iter().map(|choice| choice.label()).collect()
    }

    fn position(self) -> u32 {
        let position = Self::ALL
            .iter()
            .position(|&choice| choice == self)
            .expect("ALL lists every choice");
        position as u32
    }

    fn at_position(position: u32) -> Option<Self> {
        Self::ALL.get(position as usize).copied()
    }
}
