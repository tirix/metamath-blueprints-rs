/* Blueprint Project */

use std::fs::{read_to_string, File};
use std::path::Path;
use std::{fs::Metadata, path::PathBuf};

use crate::item::Item;
use crate::nav::Nav;
use crate::Result;
use crate::{ensure_dir, get_cmark_options, get_hbs_templates, traverse, Error, OUT_DIR};
use pulldown_cmark::Parser;
use serde::Serialize;

#[derive(Serialize)]
pub struct ProjectPage<'a> {
    project: &'a Project,
    nav: &'a Nav<'a>,
    dependencies: String,
}

#[derive(Serialize)]
pub struct Project {
    name: String,
    path: PathBuf,
    pub(crate) items: Vec<Item>,
    body: String,
}

impl Project {
    pub fn from_path(path: &PathBuf, md: Metadata, name: &str) -> Result<Option<Self>> {
        if md.is_dir() && name != OUT_DIR && !name.starts_with('.') {
            let readme_file = path.join("README.md");
            if !readme_file.exists() {
                Err(Error::Custom(format!(
                    "No README.md file for project {}",
                    name
                )))?;
            }
            let markdown_part = read_to_string(readme_file)?;
            let parser = Parser::new_ext(&markdown_part, *get_cmark_options());
            let mut body = String::new();
            pulldown_cmark::html::push_html(&mut body, parser);
            Ok(Some(Project {
                name: name.to_string(),
                path: path.to_path_buf(),
                items: traverse(path, Item::from_path)?,
                body,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn build(&self, nav: &Nav, out_path: &Path) -> Result {
        println!("Building: {}", self.name);
        let project_out_path = ensure_dir(out_path, &self.name)?;
        for item in &self.items {
            item.build(nav, self, &project_out_path)?;
        }
        self.render_project_page(nav, &project_out_path)?;
        Ok(())
    }

    fn render_project_page(&self, nav: &Nav, out_path: &Path) -> Result {
        let output_path = out_path.join("index.html");
        println!("Writing {:?}", &output_path);
        let output_file = File::create(&output_path)?;
        let dependencies = self.render_dependency_graph()?;
        get_hbs_templates()?.render_to_write(
            "project",
            &ProjectPage {
                project: self,
                nav,
                dependencies,
            },
            output_file,
        )?;
        Ok(())
    }

    fn _get_item(&self, name: &str) -> Option<&Item> {
        self.items.iter().find(|i| i.name == name)
    }
}
