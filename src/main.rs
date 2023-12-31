mod program_errors;
mod save_information;
mod save_type;
mod saves;

use std::{
    collections::HashMap,
    env,
    ffi::OsString,
    fs,
    io::{stdin, stdout, Write},
    path::{Path, PathBuf},
};

use clap::Parser;
use program_errors::ProgramError;
use save_information::SaveInformation;
use save_type::SaveType;
use saves::Saves;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct ProgramConfig {
    /// The path the program should run against.
    #[arg(short, long)]
    path_to_save_folder: Option<OsString>,

    /// The latest n saves to ignore when selecting saves for deletion
    #[arg(short, long)]
    saves_to_preserve: Option<usize>,
}

const DEFAULT_SAVES_TO_PRESERVE: usize = 10;

fn main() -> Result<(), ProgramError> {
    let program_config = ProgramConfig::parse();
    let saves_to_preserve = program_config
        .saves_to_preserve
        .unwrap_or(DEFAULT_SAVES_TO_PRESERVE);

    let directory = path_to_use(program_config.path_to_save_folder)?;

    println!(
        "Running program with saves_to_preserve: {} and path: {}",
        &saves_to_preserve,
        &directory.to_str().unwrap() // unwrap?
    );

    match fs::read_dir(directory.clone())
        .map_err(|e| ProgramError::CannotReadDirectory(e.to_string()))
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
                    // Filter empty string folders and non ascii names.
                    !dir_entry.file_name().is_empty() && dir_entry.file_name().is_ascii()
                })
                // Parse each directory
                .flat_map(|dir_entry| {
                    dir_entry
                        .file_name()
                        .to_str()
                        .ok_or(ProgramError::AsciiErrorInFileName(
                            "Unable to get ascii string from OsString".to_string(),
                        ))
                        .and_then(crate::package_details)
                }) // Up to this point errors only affect individual folders, ignore errors as those folders will be dropped and continue.
                .collect::<Vec<SaveInformation>>()
        })
        .map(crate::group_saves) // Here errors start to matter for the set, don't drop and output below.
        .map(crate::sort_map_saves)
        .map(|map| get_delete_vec(map, saves_to_preserve))
        .map(crate::confirm_user_delete)
        .and_then(|(deletable_saves, user_input)| delete((deletable_saves, user_input, directory)))
    {
        Ok(_) => (),
        Err(e) => {
            println!("Encountered error:");
            println!("{}", e);
        }
    };

    Ok(())
}

fn path_to_use(given_path: Option<OsString>) -> Result<PathBuf, ProgramError> {
    match given_path {
        Some(path) => Ok(PathBuf::from(path)),
        None => env::current_dir().map_err(|e| ProgramError::NoPath(e.to_string())),
    }
}

fn package_details(file_name: &str) -> Result<SaveInformation, ProgramError> {
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

fn character_name(folder_name: &str) -> Result<String, ProgramError> {
    folder_name
        .find('-')
        .filter(|index| index > &0)
        .map(|index| folder_name.chars().take(index).collect())
        .ok_or(ProgramError::NameNotDetected(
            "Could not detect character name".to_string(),
        ))
}

fn save_number(folder_name: &str) -> Result<u16, ProgramError> {
    let folder_name: Vec<&str> = folder_name.split('_').collect();

    if folder_name.len() <= 1 {
        return Err(ProgramError::NotEnoughUnderscores(
            "Did not find the correct number of underscores. Cannot continue with this save."
                .to_string(),
        ));
    }

    folder_name
        .into_iter()
        .last()
        .ok_or(ProgramError::NotEnoughUnderscores(
            "Could not find any elements".to_string(),
        ))
        .and_then(|save_number| {
            save_number
                .parse::<u16>()
                .map_err(|e| ProgramError::StringNotNumber(e.to_string()))
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
    let saves = map
        .entry(save_information.character_name.to_string())
        .or_insert_with(Saves::new);

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
                // The grouping into a map is to apply number_to_preserve to each character as well as
                // quick and auto saves for each character.
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
    saves.into_iter().skip(number_to_preserve).collect()
}

fn confirm_user_delete(deletable_saves: Vec<SaveInformation>) -> (Vec<SaveInformation>, String) {
    println!("****");
    deletable_saves
        .iter()
        .enumerate()
        .for_each(|(i, save)| println!("\t{} | {}", i + 1, &save.file_name));
    println!("****");

    print!("Delete the above files? y/n: ");
    let _ = stdout().flush();

    let mut user_input = String::new();
    let _input = stdin().read_line(&mut user_input);
    user_input = user_input.trim().to_string();
    println!("User input read: {}", &user_input);

    (deletable_saves, user_input)
}

fn delete(
    (deletable_saves, user_input, dir_to_use): (Vec<SaveInformation>, String, PathBuf),
) -> Result<Vec<()>, ProgramError> {
    if !user_input.eq_ignore_ascii_case("y") {
        println!("User did not confirm delete");
        return Ok(Vec::new());
    }

    deletable_saves
        .into_iter()
        .map(move |save_information| {
            let mut c = dir_to_use.clone().into_os_string();
            c.push(format!("/{}", save_information.file_name));

            c.into()
        })
        // Remove children in the directory and then remove the directory itself.
        .map(|path: PathBuf| {
            remove_children_of_dir(&path).and_then(|_| {
                fs::remove_dir(path).map_err(|e| ProgramError::FailedToDelete(e.to_string()))
            })
        })
        .collect::<Result<Vec<()>, ProgramError>>()
}

fn remove_children_of_dir(path: &impl AsRef<Path>) -> Result<Vec<()>, ProgramError> {
    fs::read_dir(path)
        .map_err(|e| ProgramError::FailedToReadDir(e.to_string()))
        .and_then(|children| {
            children
                .flatten()
                .map(|child| child.path())
                .map(|child_path: PathBuf| {
                    fs::remove_file(child_path)
                        .map_err(|e| ProgramError::FailedToDelete(e.to_string()))
                })
                .collect::<Result<Vec<()>, ProgramError>>() // TODO: Review. This is going to drop some errors silently.
        })
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
    use crate::{character_name, ProgramError};

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
        let expected = ProgramError::NameNotDetected("Could not detect character name".to_string());

        let error = character_name(test_save).unwrap_err();
        assert_eq!(error, expected);
    }
}

#[cfg(test)]
mod save_number_should {
    use rand::Rng;

    use crate::{save_number, ProgramError};

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
        let expected = ProgramError::StringNotNumber("invalid digit found in string".to_string());

        let result = save_number(&test_save).unwrap_err();
        assert_eq!(result, expected);
    }

    #[test]
    fn error_when_no_underscores() {
        let test_save = "Some'me";
        let expected = ProgramError::NotEnoughUnderscores(
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
                .find(|save_information| save_information.save_type == SaveType::Quick)
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
        assert_eq!(result.first().unwrap(), saves.get(1).unwrap());
        assert_eq!(result.get(1).unwrap(), saves.get(2).unwrap());
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

    #[test]
    fn return_empty_vec_when_preserves_all() {
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

        let result = deletable_saves(saves, 5usize);
        assert!(
            result.is_empty(),
            "Vector was not empty when asked to preserve more saves than were present"
        );
    }
}
