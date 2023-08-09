use std::{collections::HashMap, env, error::Error, fmt::Display, fs, hash::Hash};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SaveType {
    Quick,
    Auto,
    Unrecognized,
}

#[derive(Debug, PartialEq, Clone)]
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

#[derive(Debug, PartialEq, Default, Clone)]
struct Saves {
    pub quick_saves: Vec<SaveInformation>,
    pub auto_saves: Vec<SaveInformation>,
}
impl Saves {
    pub fn new() -> Self {
        Saves {
            quick_saves: Vec::new(),
            auto_saves: Vec::new(),
        }
    }
}

// TODO: Rename
#[derive(Debug, PartialEq)]
enum SelfErrors {
    NameNotDetected(String),
    NotEnoughUnderscores(String),
    StringNotNumber(String),
    AsciiErrorInFileName(String),
}
impl Error for SelfErrors {}
impl Display for SelfErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SelfErrors::NameNotDetected(e) => write!(f, "{:#?}", e),
            SelfErrors::NotEnoughUnderscores(e) => write!(f, "{:#?}", e),
            SelfErrors::StringNotNumber(e) => write!(f, "{:#?}", e),
            SelfErrors::AsciiErrorInFileName(e) => write!(f, "{:#?}", e),
        }
    }
}

fn main() {
    match env::current_dir()
        .and_then(fs::read_dir)
        .and_then(|dir_entries| {
            // TODO: Split into functions and test?
            Ok(
                dir_entries
                    .flatten()
                    .filter(|dir_entry| {
                        dir_entry
                            .file_type()
                            .map(|file_type| file_type.is_dir())
                            .unwrap_or(false)
                    })
                    .filter(|dir_entry| {
                        // Filter empty string folders and non ascii names.
                        !dir_entry.file_name().is_empty() && dir_entry.file_name().is_ascii()
                    })
                    // Parse each directory
                    .map(|dir_entry| {
                        dir_entry
                            .file_name()
                            .to_str()
                            .ok_or(SelfErrors::AsciiErrorInFileName(
                                "Unable to get ascii string from OsString".to_string(),
                            ))
                            .and_then(crate::package_details)
                    })
                    .flatten() // Up to this point errors only affect individual folders, ignore errors as those folders will be dropped and continue.
                    .collect::<Vec<SaveInformation>>(), // Done doing for each logic, collect into vector for grouping.
            )
        })
        .map(crate::group_saves) // Here errors start to matter for the set, don't drop and output below.
        .map(crate::sort_map_saves)
    {
        Ok(map) => println!("{:#?}", map),
        Err(e) => {
            println!("Encountered error:");
            println!("{:#?}", e);
        }
    };
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
    let folder_name: Vec<&str> = folder_name.split('_').collect();

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

fn group_saves(saves: Vec<SaveInformation>) -> HashMap<String, Saves> {
    saves
        .into_iter()
        .fold(HashMap::new(), crate::group_by_character)
}

fn group_by_character(
    mut map: HashMap<String, Saves>,
    save_information: SaveInformation,
) -> HashMap<String, Saves> {
    // TODO: I think there's a better way. Come back to this.
    let saves = match map.get_mut(&save_information.character_name) {
        Some(saves) => saves, // Saves already exists
        None => {
            // Need to create Saves struct for map
            let save_by_type = Saves::new();
            map.insert(save_information.character_name.clone(), save_by_type);
            map.get_mut(&save_information.character_name).unwrap()
        }
    };

    insert_save(saves, save_information);

    map
}

fn insert_save(save_by_type: &mut Saves, save_information: SaveInformation) {
    match &save_information.save_type {
        SaveType::Quick => save_by_type.quick_saves.push(save_information),
        SaveType::Auto => save_by_type.auto_saves.push(save_information),
        SaveType::Unrecognized => {}
    };
}

fn sort_map_saves(mut map: HashMap<String, Saves>) -> HashMap<String, Saves> {
    map.values_mut().for_each(|value| {
        value
            .quick_saves
            .sort_by(|save_a, save_b| save_b.save_number.partial_cmp(&save_a.save_number).unwrap());

        value
            .auto_saves
            .sort_by(|save_a, save_b| save_b.save_number.partial_cmp(&save_a.save_number).unwrap())
    });

    map
}

fn get_delete_vec(map: HashMap<String, Saves>, number_to_preserve: usize) -> Vec<SaveInformation> {
    map.into_iter()
        .fold(Vec::new(), |deletion_saves, (_, character_saves)| {
            deletion_saves
                .into_iter()
                // Combine existing saves to be deleted with those detected deletable_saves.
                // The grouping into a map is to apply number_to_preserve to both each character as well as
                // quick and auto saves.
                .chain(
                    deletable_saves(character_saves.quick_saves, number_to_preserve)
                        .into_iter()
                        .chain(
                            deletable_saves(character_saves.auto_saves, number_to_preserve)
                                .into_iter(),
                        ),
                )
                .collect()
        })
}

fn deletable_saves(saves: Vec<SaveInformation>, number_to_preserve: usize) -> Vec<SaveInformation> {
    if saves.len() <= number_to_preserve {
        return Vec::new();
    }

    saves.into_iter().skip(number_to_preserve).collect()
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

        let result = save_number(test_save).unwrap_err();
        assert_eq!(result, expected);
    }
}

#[cfg(test)]
mod group_by_character_should {
    use std::collections::HashMap;

    use crate::{group_by_character, group_saves, SaveInformation, SaveType};

    #[test]
    fn create_and_assign_new_character_quicksave() {
        let map = HashMap::default();
        let character_name = "First Last".to_string();
        let save_information =
            SaveInformation::new_random(SaveType::Quick, character_name.to_string());
        let expected = save_information.clone();

        let map = group_by_character(map, save_information);

        let save_by_type = map.get(&character_name).unwrap();
        assert_eq!(save_by_type.quick_saves.len(), 1);
        assert_eq!(save_by_type.quick_saves.first().unwrap(), &expected);
    }

    #[test]
    fn create_and_assign_new_character_autosave() {
        let map = HashMap::default();
        let character_name = "First Last".to_string();
        let save_information = SaveInformation::new_random(SaveType::Auto, character_name.clone());
        let expected = save_information.clone();

        let map = group_by_character(map, save_information);

        let save_by_type = map.get(&character_name).unwrap();
        assert_eq!(save_by_type.auto_saves.len(), 1);
        assert_eq!(save_by_type.auto_saves.first().unwrap(), &expected);
    }

    #[test]
    fn multiple_saves_of_single_character() {
        let map = HashMap::default();
        let character_name = "First Last".to_string();

        let save_informations = vec![
            SaveInformation::new_random(SaveType::Quick, character_name.clone()),
            SaveInformation::new_random(SaveType::Quick, character_name.clone()),
            SaveInformation::new_random(SaveType::Quick, character_name.clone()),
        ];

        let map = group_by_character(map, save_informations.first().unwrap().clone());
        assert_eq!(map.get(&character_name).unwrap().quick_saves.len(), 1);
        assert_eq!(
            map.get(&character_name)
                .unwrap()
                .quick_saves
                .first()
                .unwrap(),
            save_informations.first().unwrap()
        );

        let map = group_by_character(map, save_informations.iter().nth(1).unwrap().clone());
        assert_eq!(map.get(&character_name).unwrap().quick_saves.len(), 2);
        assert_eq!(
            map.get(&character_name)
                .unwrap()
                .quick_saves
                .last()
                .unwrap(),
            save_informations.iter().nth(1).unwrap()
        );

        let map = group_by_character(map, save_informations.last().unwrap().clone());
        assert_eq!(map.get(&character_name).unwrap().quick_saves.len(), 3);
        assert_eq!(
            map.get(&character_name)
                .unwrap()
                .quick_saves
                .last()
                .unwrap(),
            save_informations.last().unwrap()
        );
    }

    #[test]
    fn multiple_saves_of_multiple_characters() {
        let (fl, some) = ("First Last".to_string(), "Some'me".to_string());

        let fl_save_information = vec![
            SaveInformation::new_random(SaveType::Quick, fl.clone()),
            SaveInformation::new_random(SaveType::Auto, fl.clone()),
            SaveInformation::new_random(SaveType::Quick, fl.clone()),
            SaveInformation::new_random(SaveType::Auto, fl.clone()),
        ];

        let some_save_information = vec![
            SaveInformation::new_random(SaveType::Quick, some.clone()),
            SaveInformation::new_random(SaveType::Auto, some.clone()),
            SaveInformation::new_random(SaveType::Quick, some.clone()),
            SaveInformation::new_random(SaveType::Auto, some.clone()),
        ];

        let map = group_saves(
            fl_save_information
                .clone()
                .into_iter()
                .chain(some_save_information.clone().into_iter())
                .collect(),
        );

        assert_eq!(map.keys().len(), 2);
        assert!(map.keys().any(|key| key.eq("First Last")));
        assert!(map.keys().any(|key| key.eq("Some'me")));

        let fl_saves = map.get(&fl).unwrap();
        let some_saves = map.get(&some).unwrap();

        for save in fl_save_information.into_iter() {
            assert!(
                match &save.save_type {
                    SaveType::Quick => fl_saves
                        .quick_saves
                        .iter()
                        .any(|quick_save| quick_save.eq(&save)),
                    SaveType::Auto => fl_saves
                        .auto_saves
                        .iter()
                        .any(|auto_save| auto_save.eq(&save)),
                    SaveType::Unrecognized => panic!("Unrecognized save type was not removed"),
                },
                "Failed to match save"
            );
        }

        for save in some_save_information.into_iter() {
            assert!(
                match &save.save_type {
                    SaveType::Quick => some_saves
                        .quick_saves
                        .iter()
                        .any(|quick_save| quick_save.eq(&save)),
                    SaveType::Auto => some_saves
                        .auto_saves
                        .iter()
                        .any(|auto_save| auto_save.eq(&save)),
                    SaveType::Unrecognized => panic!("Unrecognized save type was not removed"),
                },
                "Failed to match save"
            );
        }
    }
}

#[cfg(test)]
mod sort_map_saves_should {

    use crate::{group_saves, sort_map_saves, SaveInformation, SaveType};

    #[test]
    fn sorts_quick_saves() {
        let mut saves = vec![
            SaveInformation::new_random(SaveType::Quick, "First Last".to_string()),
            SaveInformation::new_random(SaveType::Quick, "First Last".to_string()),
        ];
        // Force ascending order
        saves
            .sort_by(|save_a, save_b| save_a.save_number.partial_cmp(&save_b.save_number).unwrap());

        let map = group_saves(saves.clone());
        let map = sort_map_saves(map);
        let fl_saves = map.get("First Last").unwrap();

        assert_eq!(fl_saves.quick_saves.first().unwrap(), saves.last().unwrap());
        assert_eq!(fl_saves.quick_saves.last().unwrap(), saves.first().unwrap());
    }

    #[test]
    fn sorts_auto_saves() {
        let mut saves = vec![
            SaveInformation::new_random(SaveType::Auto, "First Last".to_string()),
            SaveInformation::new_random(SaveType::Auto, "First Last".to_string()),
        ];
        // Force ascending order
        saves
            .sort_by(|save_a, save_b| save_a.save_number.partial_cmp(&save_b.save_number).unwrap());

        let map = group_saves(saves.clone());
        let map = sort_map_saves(map);
        let fl_saves = map.get("First Last").unwrap();

        assert_eq!(fl_saves.auto_saves.first().unwrap(), saves.last().unwrap());
        assert_eq!(fl_saves.auto_saves.last().unwrap(), saves.first().unwrap());
    }
}

#[cfg(test)]
mod get_delete_vec_should {
    use std::collections::HashMap;

    use crate::{get_delete_vec, SaveInformation, SaveType, Saves};

    #[test]
    fn handle_quick_and_auto_saves() {
        let mut map = HashMap::new();
        let name = "First Last".to_string();

        let quick_saves = vec![
            SaveInformation::new_random(SaveType::Quick, name.to_string()),
            SaveInformation::new_random(SaveType::Quick, name.to_string()),
        ];
        let auto_saves = vec![
            SaveInformation::new_random(SaveType::Auto, name.to_string()),
            SaveInformation::new_random(SaveType::Auto, name.to_string()),
        ];

        map.insert(
            name.to_string(),
            Saves {
                quick_saves: quick_saves.clone(),
                auto_saves: auto_saves.clone(),
            },
        );

        let result = get_delete_vec(map.clone(), 1usize);
        assert_eq!(result.len(), 2);
        assert_eq!(
            result
                .iter()
                .filter(|save_information| save_information.save_type == SaveType::Quick)
                .count(),
            1
        );
        assert_eq!(
            result
                .iter()
                .filter(|save_information| save_information.save_type == SaveType::Auto)
                .count(),
            1
        );

        assert_eq!(
            result
                .iter()
                .filter(|save_information| save_information.save_type == SaveType::Quick)
                .nth(0)
                .unwrap()
                .clone(),
            quick_saves.iter().nth(1).unwrap().clone()
        )
    }
}

#[cfg(test)]
mod deletable_saves_should {
    use rand::Rng;

    use crate::{deletable_saves, SaveInformation, SaveType};

    #[test]
    fn return_correct_saves_from_fixed_pool() {
        let saves = vec![
            SaveInformation::new(
                "test_file_name1".to_string(),
                "First Last".to_string(),
                SaveType::Auto,
                33u16,
            ),
            SaveInformation::new(
                "test_file_name2".to_string(),
                "First Last".to_string(),
                SaveType::Auto,
                32u16,
            ),
            SaveInformation::new(
                "test_file_name3".to_string(),
                "First Last".to_string(),
                SaveType::Auto,
                31u16,
            ),
        ];

        let result = deletable_saves(saves.clone(), 1usize);
        assert_eq!(result.len(), 2);
        assert_eq!(result.first().unwrap(), saves.iter().nth(1).unwrap());
        assert_eq!(result.iter().nth(1).unwrap(), saves.iter().nth(2).unwrap());
    }

    #[test]
    fn return_correct_saves_from_randomized_pool() {
        let mut saves = Vec::new();

        let number_to_generate = rand::thread_rng().gen_range(10..1002);

        for _ in 0..number_to_generate {
            saves.push(SaveInformation::new_random(
                SaveType::Quick,
                "First Last".to_string(),
            ));
        }

        let number_to_preserve = rand::thread_rng().gen_range(1..saves.len() - 5);
        let result = deletable_saves(saves.clone(), number_to_preserve);

        assert_ne!(result.len(), saves.len());
        assert_eq!(result.len(), number_to_generate - number_to_preserve);
    }
}
