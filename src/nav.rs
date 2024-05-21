use crate::project::Project;
use crate::{get_cmark_options, get_hbs_templates, Error, Result};
use pulldown_cmark::Parser;
use serde::Serialize;
use std::fs::{read_to_string, File};
use std::path::Path;

#[derive(Serialize)]
pub struct IndexPage<'a> {
    body: String,
    nav: &'a Nav<'a>,
}

#[derive(Serialize)]
pub struct Nav<'a> {
    pub(crate) projects: &'a Vec<Project>,
}

impl<'a> Nav<'a> {
    fn body(&self, path: &Path) -> Result<String> {
        let readme_file = path.join("README.md");
        if !readme_file.exists() {
            Err(Error::Custom("No README.md file for index".to_string()))?;
        }
        let markdown_part = read_to_string(readme_file)?.replace("# Metamath Blueprints", "");
        let parser = Parser::new_ext(&markdown_part, *get_cmark_options());
        let mut body = String::new();
        pulldown_cmark::html::push_html(&mut body, parser);
        Ok(body)
    }

    pub fn build(&self, path: &Path, out_path: &Path) -> Result {
        println!("Building Index");
        let body = self.body(path)?;
        let output_path = out_path.join("index.html");
        let output_file = File::create(output_path)?;
        get_hbs_templates()?.render_to_write(
            "index",
            &IndexPage { body, nav: self },
            output_file,
        )?;
        Ok(())
    }
}
