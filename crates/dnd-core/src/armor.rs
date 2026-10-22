//! Armor table from the SRD 5.2.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArmorKind {
    Padded,
    Leather,
    StuddedLeather,
    Hide,
    ChainShirt,
    ScaleMail,
    Breastplate,
    HalfPlate,
    RingMail,
    ChainMail,
    Splint,
    Plate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmorCategory {
    Light,
    Medium,
    Heavy,
}

pub struct ArmorInfo {
    pub name: &'static str,
    pub base_ac: i32,
    pub category: ArmorCategory,
    pub stealth_disadvantage: bool,
    pub strength_required: i32,
}

impl ArmorKind {
    pub const ALL: [ArmorKind; 12] = [
        ArmorKind::Padded,
        ArmorKind::Leather,
        ArmorKind::StuddedLeather,
        ArmorKind::Hide,
        ArmorKind::ChainShirt,
        ArmorKind::ScaleMail,
        ArmorKind::Breastplate,
        ArmorKind::HalfPlate,
        ArmorKind::RingMail,
        ArmorKind::ChainMail,
        ArmorKind::Splint,
        ArmorKind::Plate,
    ];

    pub fn info(self) -> ArmorInfo {
        use ArmorCategory::*;
        let (name, base_ac, category, stealth_disadvantage, strength_required) = match self {
            ArmorKind::Padded => ("Padded Armor", 11, Light, true, 0),
            ArmorKind::Leather => ("Leather Armor", 11, Light, false, 0),
            ArmorKind::StuddedLeather => ("Studded Leather Armor", 12, Light, false, 0),
            ArmorKind::Hide => ("Hide Armor", 12, Medium, false, 0),
            ArmorKind::ChainShirt => ("Chain Shirt", 13, Medium, false, 0),
            ArmorKind::ScaleMail => ("Scale Mail", 14, Medium, true, 0),
            ArmorKind::Breastplate => ("Breastplate", 14, Medium, false, 0),
            ArmorKind::HalfPlate => ("Half Plate Armor", 15, Medium, true, 0),
            ArmorKind::RingMail => ("Ring Mail", 14, Heavy, true, 0),
            ArmorKind::ChainMail => ("Chain Mail", 16, Heavy, true, 13),
            ArmorKind::Splint => ("Splint Armor", 17, Heavy, true, 15),
            ArmorKind::Plate => ("Plate Armor", 18, Heavy, true, 15),
        };
        ArmorInfo {
            name,
            base_ac,
            category,
            stealth_disadvantage,
            strength_required,
        }
    }

    /// AC from this armor given a Dexterity modifier.
    pub fn ac(self, dex_mod: i32) -> i32 {
        let info = self.info();
        info.base_ac
            + match info.category {
                ArmorCategory::Light => dex_mod,
                ArmorCategory::Medium => dex_mod.min(2),
                ArmorCategory::Heavy => 0,
            }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn armor_dex_caps() {
        assert_eq!(ArmorKind::StuddedLeather.ac(4), 16);
        assert_eq!(ArmorKind::HalfPlate.ac(4), 17);
        assert_eq!(ArmorKind::Breastplate.ac(-1), 13);
        assert_eq!(ArmorKind::Plate.ac(-1), 18);
        assert_eq!(ArmorKind::Plate.ac(3), 18);
    }
}
