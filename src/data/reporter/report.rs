use std::fmt::{self, Display};

use crate::data::models::file::DataFile;

#[derive(Debug)]
pub struct Report {
    pub files: Vec<FileReport>,
}

impl Report {
    pub fn new() -> Self { Self { files: vec![] } }

    pub fn from_files(files: Vec<FileReport>) -> Self {
        Self { files }
    }

    pub fn add_set(&mut self, file_report: FileReport) {
        self.files.push(file_report);
    }
}

impl Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for file in &self.files {
            write!(f, "{}\n", file)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct FileReport {
    pub file_name: String,
    pub sets: Vec<SetReport>,
    pub unknown: Vec<String>
}

impl FileReport {
    pub fn new(file_name: String) -> Self { Self { file_name, sets: vec![], unknown: vec![] } }
    pub fn add_set(&mut self, set: SetReport) {
        self.sets.push(set);
    }
    pub fn add_unknown(&mut self, unknown: String) {
        self.unknown.push(unknown);
    }
}


impl Display for FileReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "File name: {}", self.file_name)?;
        for s in &self.sets {
            writeln!(f, "- Set: {}", s)?;
        }
        if self.unknown.len() > 0 {
            writeln!(f, "Unknown files:")?;
        }
        for file in &self.unknown {
            writeln!(f, "- {}", file)?;
        }

        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct SetReport {
    pub name: String,
    pub roms_have: Vec<DataFile>,
    pub roms_to_rename: Vec<FileRename>,
    pub roms_missing: Vec<DataFile>,
    pub roms_unneeded: Vec<DataFile>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum SetNameReport {
    Name(String),
    RenameFromTo(String, String)
}

impl SetNameReport {
    pub fn new(set_name: String, reference_name: String) -> Self {
        if set_name.eq(&reference_name) {
            SetNameReport::Name(set_name)
        } else {
            SetNameReport::RenameFromTo(set_name, reference_name)
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct FileRename {
    pub from: DataFile,
    pub to: String,
}

impl FileRename {
    pub fn new(from: DataFile, to: String) -> Self { Self { from, to } }
}

impl SetReport {
    pub fn new(name: String) -> Self {
        Self {
            name,
            roms_have: vec![],
            roms_to_rename: vec![],
            roms_missing: vec![],
            roms_unneeded: vec![],
        }
    }

    pub fn from_data(name: String, roms_have: Vec<DataFile>, roms_to_rename: Vec<FileRename>, roms_missing: Vec<DataFile>, roms_unneeded: Vec<DataFile>) -> Self {
        Self { name, roms_have, roms_to_rename, roms_missing, roms_unneeded }
    }

    pub fn add_having(&mut self, rom: DataFile) {
        self.roms_have.push(rom);
    }

    pub fn add_missing(&mut self, rom: DataFile) {
        self.roms_missing.push(rom);
    }
}


impl Display for SetReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut output = format!("{}", self.name);

        if self.roms_have.len() > 0 {
            output.push_str("\nRoms:");
            for have in self.roms_have.as_slice() {
                output.push_str(&format!("\n    - {}", have));
            }
        }

        if self.roms_to_rename.len() > 0 {
            output.push_str("\nTo Rename:");
            for to_rename in self.roms_to_rename.as_slice() {
                output.push_str(&format!("\n    - {} => {}", to_rename.from, to_rename.to));
            }
        }

        if self.roms_missing.len() > 0 {
            output.push_str("\nMissing:");
            for missing in self.roms_missing.as_slice() {
                output.push_str(&format!("\n    - {}", missing));
            }
        }

        if self.roms_unneeded.len() > 0 {
            output.push_str("\nUnneeded:");
            for unneeded in self.roms_unneeded.as_slice() {
                output.push_str(&format!("\n    - {}", unneeded));
            }
        }

        write!(f, "{}", output)
    }
}