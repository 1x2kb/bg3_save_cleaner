use crate::save_type::SaveType;

#[derive(Debug, PartialEq, Clone)]
pub struct SaveInformation {
    pub file_name: String,
    pub character_name: String,
    pub save_type: SaveType,
    pub save_number: u16,
}
impl SaveInformation {
    pub fn new(
        file_name: String,
        character_name: String,
        save_type: SaveType,
        save_number: u16,
    ) -> Self {
        SaveInformation {
            file_name,
            character_name,
            save_type,
            save_number,
        }
    }

    #[cfg(test)]
    pub fn new_random(save_type: SaveType, character_name: String) -> Self {
        use rand::Rng;

        let save_number = rand::thread_rng().gen_range(u16::MIN..=u16::MAX);

        match save_type {
            SaveType::Quick => SaveInformation {
                file_name: format!("{}-123456789__QuickSave_{}", character_name, save_number),
                character_name,
                save_type,
                save_number,
            },
            SaveType::Auto => SaveInformation {
                file_name: format!("{}-123456789__AutoSave_{}", character_name, save_number),
                character_name,
                save_type,
                save_number,
            },
            _ => panic!("Not a randomizable save pattern"),
        }
    }
}
