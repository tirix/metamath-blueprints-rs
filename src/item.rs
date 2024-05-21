/* A Blueprint Item */

use std::fs::{read_to_string, File};
use std::path::Path;
use std::{fs::Metadata, path::PathBuf};

use crate::nav::Nav;
use crate::project::Project;
use crate::Result;
use crate::{get_cmark_options, get_hbs_templates, Error};
use pulldown_cmark::Parser;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub enum ItemType {
    #[default]
    Theorem,
    Definition,
}

#[derive(Serialize, Deserialize, Default)]
pub enum ItemState {
    #[default]
    Draft,
    ReadyForStmt,
    StmtFormalized,
    ReadyForProof,
    Formalized,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ItemInfo {
    pub r#type: ItemType,
    pub state: ItemState,
    pub statement: Option<String>,
    pub dependencies: Vec<String>,
    pub hide: bool,
    pub reference: Option<String>,
    pub wikipedia: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Item {
    pub name: String,
    path: PathBuf,
    pub info: ItemInfo,
    body: String,
}

#[derive(Serialize)]
pub struct ItemPage<'a> {
    project: &'a Project,
    item: &'a Item,
    nav: &'a Nav<'a>,
}

impl Item {
    pub fn from_path(path: &PathBuf, md: Metadata, name: &str) -> Result<Option<Self>> {
        if md.is_dir() || name == "README" {
            Ok(None)
        } else {
            let mut body = String::new();
            let matter_input = read_to_string(path)?;
            let (matter_part, markdown_part) = matter::matter(&matter_input).ok_or(
                Error::Custom(format!("Could not parse front matter for {}", name)),
            )?;
            let parser = Parser::new_ext(&markdown_part, *get_cmark_options());
            let info: ItemInfo = toml::from_str(&matter_part)?;
            if info.hide {
                Ok(None)
            } else {
                pulldown_cmark::html::push_html(&mut body, parser);
                Ok(Some(Item {
                    name: name.to_string(),
                    path: path.clone(),
                    info,
                    body,
                }))
            }
        }
    }

    pub fn build(&self, nav: &Nav, parent: &Project, out_path: &Path) -> Result {
        let output_path = out_path.join(self.name.to_string() + ".html");
        println!("Writing {:?}", &output_path);
        let output_file = File::create(&output_path)?;
        get_hbs_templates()?.render_to_write(
            "item",
            &ItemPage {
                project: parent,
                item: self,
                nav,
            },
            output_file,
        )?;
        Ok(())
    }
}
