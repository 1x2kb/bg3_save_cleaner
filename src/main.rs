use std::{
    env,
    fs::{self, DirEntry},
};

#[derive(Debug, PartialEq, Eq)]
enum SaveType {
    Quick,
    Auto,
    Unrecognized,
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

fn save_type<'a>(folder_name: &str) -> SaveType {
    if folder_name.to_ascii_lowercase().contains("quicksave") {
        SaveType::Quick
    } else if folder_name.to_ascii_lowercase().contains("autosave") {
        SaveType::Auto
    } else {
        SaveType::Unrecognized
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
