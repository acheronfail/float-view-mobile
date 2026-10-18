use crate::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BoardAppearance {
    pub rail: u8,
    pub tyre: u8,
    pub fender: u8,
    pub hooks: u8,
    pub left_top: String,
    pub left_bottom: String,
    pub right_top: String,
    pub right_bottom: String,
    pub rubber: String,
    pub rim: String,
    pub fenders: String,
    pub hook_colour: String,
}
impl Default for BoardAppearance {
    fn default() -> Self {
        Self {
            rail: 1,
            tyre: 1,
            fender: 1,
            hooks: 1,
            left_top: "#a9afb8".into(),
            left_bottom: "#a9afb8".into(),
            right_top: "#a9afb8".into(),
            right_bottom: "#a9afb8".into(),
            rubber: "#59616b".into(),
            rim: "#17191d".into(),
            fenders: "#a9afb8".into(),
            hook_colour: "#ffffff".into(),
        }
    }
}
impl BoardAppearance {
    pub fn validate(&self) -> Result<()> {
        const COLOURS: [&str; 22] = [
            "#17191d", "#59616b", "#a9afb8", "#e4e7eb", "#ffffff", "#ed5555", "#db8a57", "#f4d84d",
            "#d4b45f", "#b4d94e", "#4dbb75", "#307653", "#38a59c", "#a2d4d8", "#4ccfe0", "#82aee0",
            "#4778e8", "#7866d8", "#b49bd4", "#e780b1", "#9b6d4b", "#d9c1a5",
        ];
        if !(1..=6).contains(&self.rail)
            || !(1..=2).contains(&self.tyre)
            || !(1..=3).contains(&self.fender)
            || !(1..=3).contains(&self.hooks)
            || (self.hooks == 2 && self.fender == 1)
            || [
                &self.left_top,
                &self.left_bottom,
                &self.right_top,
                &self.right_bottom,
                &self.rubber,
                &self.rim,
                &self.fenders,
                &self.hook_colour,
            ]
            .iter()
            .any(|colour| !COLOURS.contains(&colour.as_str()))
        {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }
}
