extern crate yaml_rust;
extern crate regex;
use regex::{Regex, Captures};
use yaml_rust::YamlLoader;
use std::io::prelude::*;
use std::error::Error;
use std::fs::File;
use std::path::Path;
use std::collections::HashMap;

static RE: &'static str = r"%\((?P<val>.+?)\)%";

#[derive(Clone)]
pub struct Action {
    pub path: String,
    pub scan: String,
    pub method: String,
    pub post_params: String,
}

#[derive(Clone)]
pub struct HakaiConfig {
    pub domain: String,
    pub user_agent: String,
    pub actions: Vec<Action>,
    pub consts: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
}

impl HakaiConfig {
    pub fn new() -> HakaiConfig {
        HakaiConfig {
            domain: "http://localhost:8888/".to_string(),
            user_agent: "rshakai/0.1".to_string(),
            actions: vec![],
            consts: HashMap::new(),
            query_params: HashMap::new(),
        }
    }

    fn parse_yaml(&mut self, yamlstr: &String) {
        let docs = YamlLoader::load_from_str(yamlstr).unwrap();
        let doc = &docs[0];

        let bad = &doc["domain"];
        if !bad.is_badvalue() {
            self.domain = bad.as_str().unwrap().to_owned();
        }

        let actions = &doc["actions"];
        if !actions.is_badvalue() {
            let actions = actions.as_vec().unwrap();
            for action in actions {
                let mut a = Action {
                    path: "".to_string(),
                    scan: "".to_string(),
                    method: "GET".to_string(),
                    post_params: "".to_string(),
                };
                for (key, value) in action.as_hash().unwrap() {
                    let k = key.as_str().unwrap();
                    if k == "method" {
                        let v = value.as_str().unwrap();
                        a.method = v.to_string();
                    } else if k == "path" {
                        let v = value.as_str().unwrap();
                        a.path = v.to_string();
                    } else if k == "post_params" {
                        // a.post_params = v.to_string();
                    }
                }
                self.actions.push(a);
            }
        }

        let consts = &doc["consts"];
        if !consts.is_badvalue() {
            let consts = consts.as_hash().unwrap();
            for (k, v) in consts {
                self.consts.insert(k.as_str().unwrap().to_string(),
                                   v.as_str().unwrap().to_string());
            }
        }

        let query_params = &doc["query_params"];
        if !query_params.is_badvalue() {
            let query_params = query_params.as_hash().unwrap();
            for (k, v) in query_params {
                self.query_params.insert(k.as_str().unwrap().to_string(),
                                         v.as_str().unwrap().to_string());
            }
        }
    }

    pub fn load(&mut self, filename: String) {
        let path = Path::new(&filename);
        let display = path.display();

        let mut f = match File::open(&path) {
            Err(why) => panic!("could not open {}: {}", display, Error::description(&why)),
            Ok(file) => file,
        };

        let mut s = String::new();
        match f.read_to_string(&mut s) {
            Err(why) => panic!("could not read {}: {}", display, Error::description(&why)),
            Ok(_) => self.parse_yaml(&s),
        };
    }
}

pub fn replace_names(input: &str, consts: &HashMap<String, String>) -> String {
    let re = Regex::new(RE).unwrap();

    let result = re.replace_all(input, |caps: &Captures| {
        let mut s = String::new();
        let org = caps.at(0).unwrap();
        let vvv = caps.at(1).unwrap();
        match consts.get(vvv) {
            Some(vv) => s.push_str(vv),
            None => s.push_str(org),
        }
        s
    });

    result
}
