use anyhow::Result;
use ignore::{
    overrides::{Override, OverrideBuilder},
    types::{Types, TypesBuilder},
};
use std::path::PathBuf;

use crate::ui::cmd_parse::SearchCmd;

#[derive(Debug)]
pub struct SearchConfig {
    pub pattern: String,
    pub path: PathBuf,
    pub case_insensitive: bool,
    pub case_smart: bool,
    pub overrides: Override,
    pub types: Types,
    pub search_hidden: bool,

    pub after_context: usize,
    pub before_context: usize,
}

impl SearchConfig {
    pub fn update_from(&mut self, cmd: SearchCmd) {
        self.pattern = cmd.pattern;
        if let Some(globs) = cmd.golb {
            if globs.len() > 0 {
                let mut builder = OverrideBuilder::new(std::env::current_dir().unwrap());
                for glob in globs {
                    let _ = builder.add(&glob);
                }
                if let Ok(ov) = builder.build() {
                    self.overrides = ov;
                }
            }
        }
        self.after_context = cmd.after_context;
        self.before_context = cmd.before_context;
    }

    pub fn from(pattern: String, path: PathBuf) -> Result<Self> {
        let mut builder = TypesBuilder::new();
        builder.add_defaults();
        let types = builder.build()?;

        Ok(Self {
            pattern,
            path,
            case_insensitive: false,
            case_smart: false,
            overrides: Override::empty(),
            types,
            search_hidden: false,
            after_context: 0,
            before_context: 0,
        })
    }

    pub fn case_insensitive(mut self, case_insensitive: bool) -> Self {
        self.case_insensitive = case_insensitive;
        self
    }

    pub fn case_smart(mut self, case_smart: bool) -> Self {
        self.case_smart = case_smart;
        self
    }

    pub fn globs(mut self, globs: Vec<String>) -> Result<Self> {
        let mut builder = OverrideBuilder::new(std::env::current_dir()?);
        for glob in globs {
            builder.add(&glob)?;
        }
        self.overrides = builder.build()?;
        Ok(self)
    }

    pub fn file_types(
        mut self,
        file_types: Vec<String>,
        file_types_not: Vec<String>,
    ) -> Result<Self> {
        let mut builder = TypesBuilder::new();
        builder.add_defaults();
        for file_type in file_types {
            builder.select(&file_type);
        }
        for file_type in file_types_not {
            builder.negate(&file_type);
        }
        self.types = builder.build()?;
        Ok(self)
    }

    pub fn search_hidden(mut self, search_hidden: bool) -> Self {
        self.search_hidden = search_hidden;
        self
    }
}
