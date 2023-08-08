use std::{env, error::Error, fmt::Display, fs};

#[derive(Debug, PartialEq, Eq)]
enum SaveType {
    Quick,
    Auto,
    Unrecognized,
}

#[derive(Debug, PartialEq)]
struct SaveInformation {
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
}

// TODO: Rename
#[derive(Debug, PartialEq)]
enum SelfErrors {
    NameNotDetected(String),
    NotEnoughUnderscores(String),
    StringNotNumber(String),
}
impl Error for SelfErrors {}
impl Display for SelfErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SelfErrors::NameNotDetected(e) => write!(f, "{:#?}", e),
            SelfErrors::NotEnoughUnderscores(e) => write!(f, "{:#?}", e),
            SelfErrors::StringNotNumber(e) => write!(f, "{:#?}", e),
        }
    }
}

fn main() {
    match env::current_dir()
        .and_then(fs::read_dir)
        .map(|dir_entries| {
            dir_entries
                .flatten()
                .filter(|dir_entry| {
                    dir_entry
                        .file_type()
                        .map(|file_type| file_type.is_dir())
                        .unwrap_or(false)
                })
                .filter(|dir_entry| {
                    !dir_entry.file_name().is_empty() && dir_entry.file_name().is_ascii()
                })
        }) {
        Ok(_) => todo!(),
        Err(e) => {
            println!("Encountered error:");
            println!("{:#?}", e);
        }
    };
}

fn save_type(folder_name: &str) -> SaveType {
    if folder_name.to_ascii_lowercase().contains("quicksave") {
        SaveType::Quick
    } else if folder_name.to_ascii_lowercase().contains("autosave") {
        SaveType::Auto
    } else {
        SaveType::Unrecognized
    }
}

fn character_name(folder_name: &str) -> Result<String, SelfErrors> {
    folder_name
        .find('-')
        .filter(|index| index > &0)
        .map(|index| folder_name.chars().take(index).collect())
        .ok_or(SelfErrors::NameNotDetected(
            "Could not detect character name".to_string(),
        ))
}

fn save_number(folder_name: &str) -> Result<u16, SelfErrors> {
    let folder_name: Vec<&str> = folder_name.split('_').into_iter().collect();

    if folder_name.len() <= 1 {
        return Err(SelfErrors::NotEnoughUnderscores(
            "Did not find the correct number of underscores. Cannot continue with this save."
                .to_string(),
        ));
    }

    folder_name
        .into_iter()
        .last()
        .ok_or(SelfErrors::NotEnoughUnderscores(
            "Could not find any elements".to_string(),
        ))
        .and_then(|save_number| {
            save_number
                .parse::<u16>()
                .map_err(|e| SelfErrors::StringNotNumber(e.to_string()))
        })
}

fn package_details(file_name: &str) -> Result<SaveInformation, SelfErrors> {
    let parse_number = save_number(file_name)?;
    let characters_name = character_name(file_name)?;
    let s_type = save_type(file_name);

    Ok(SaveInformation::new(
        file_name.to_string(),
        characters_name,
        s_type,
        parse_number,
    ))
}

#[cfg(test)]
mod save_type_should {
    use crate::{save_type, SaveType};

    #[test]
    fn detect_quick_save() {
        let test_save_type: &str = "QuickSave";
        let save = format!("Some Name-1231415123_{}_277", test_save_type);

        let save_type = save_type(save.as_str());
        assert_eq!(save_type, SaveType::Quick);
    }

    #[test]
    fn detect_auto_save() {
        let test_save_type: &str = "AutoSave";
        let save = format!("Some Name-1231415123_{}_277", test_save_type);

        let save_type = save_type(save.as_str());
        assert_eq!(save_type, SaveType::Auto);
    }

    #[test]
    fn detect_unrecognized() {
        let save = "Some Name-ManualSave";

        let save_type = save_type(save);
        assert_eq!(save_type, SaveType::Unrecognized);
    }
}

#[cfg(test)]
mod character_name_should {
    use crate::{character_name, SelfErrors};

    #[test]
    fn detect_with_space() {
        let test_save = "Some Name-1231415123_{}_277";
        let expected = "Some Name";

        let name = character_name(test_save).unwrap();
        assert_eq!(name, expected);
    }

    #[test]
    fn detect_with_underscore() {
        let test_save = "Some_Name-1231415123_{}_277";
        let expected = "Some_Name";

        let name = character_name(test_save).unwrap();
        assert_eq!(name, expected);
    }

    #[test]
    fn detect_single_name() {
        let test_save = "SomeName-1231415123_{}_277";
        let expected = "SomeName";

        let name = character_name(test_save).unwrap();
        assert_eq!(name, expected);
    }

    #[test]
    fn detect_with_apostrophe() {
        let test_save = "Some'me-1231415123_{}_277";
        let expected = "Some'me";

        let name = character_name(test_save).unwrap();
        assert_eq!(name, expected);
    }

    #[test]
    fn error_when_no_dashes() {
        let test_save = "Some'me";
        let expected = SelfErrors::NameNotDetected("Could not detect character name".to_string());

        let error = character_name(test_save).unwrap_err();
        assert_eq!(error, expected);
    }
}

#[cfg(test)]
mod save_number_should {
    use rand::Rng;

    use crate::{save_number, SelfErrors};

    #[test]
    fn convert_max_number() {
        let test_save = format!("Some'me-1231415123_QuickSave_{}", u16::MAX);

        let result = save_number(&test_save).unwrap();
        assert_eq!(result, u16::MAX);
    }

    #[test]
    fn convert_minimum_number() {
        let test_save = format!("Some'me-1231415123_QuickSave_{}", u16::MIN);

        let result = save_number(&test_save).unwrap();
        assert_eq!(result, u16::MIN);
    }

    #[test]
    fn convert_random_number() {
        let random_u16: u16 = rand::thread_rng().gen_range(1..u16::MAX); // Ignore min and max as that is explicitly tested.
        let test_save = format!("Some'me-1231415123_QuickSave_{}", random_u16);

        let result = save_number(&test_save).unwrap();
        assert_eq!(result, random_u16);
    }

    #[test]
    fn error_on_negative_number() {
        let test_save = format!("Some'me-1231415123_QuickSave_{}", -22);
        let expected = SelfErrors::StringNotNumber("invalid digit found in string".to_string());

        let result = save_number(&test_save).unwrap_err();
        assert_eq!(result, expected);
    }

    #[test]
    fn error_when_no_underscores() {
        let test_save = "Some'me";
        let expected = SelfErrors::NotEnoughUnderscores(
            "Did not find the correct number of underscores. Cannot continue with this save."
                .to_string(),
        );

        let result = save_number(&test_save).unwrap_err();
        assert_eq!(result, expected);
    }
}

#[cfg(test)]
mod package_details_should {
    use rand::Rng;

    use crate::{package_details, SaveInformation, SaveType};

    #[test]
    fn package_values_returned() {
        let rand = rand::thread_rng().gen_range(u16::MIN..=u16::MAX);
        let test_save = format!("Some'me-1231415123_QuickSave_{}", rand);

        let expected = SaveInformation::new(
            test_save.clone(),
            "Some'me".to_string(),
            SaveType::Quick,
            rand,
        );

        let result = package_details(test_save.as_str()).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn errors_out_when_error_state_occurs() {
        let test_save = "Some'me";

        let result = package_details(test_save);
        assert!(
            result.is_err(),
            "Package did not error when it was provided insufficient information"
        );
    }
}
