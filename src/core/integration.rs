use serde::{Deserialize, Serialize};

pub const PATH_SEPARATOR: char = '/';

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Param {
    pub name: String,
    pub value: String,
}

impl Param {
    pub fn new(name: String, value: String) -> Self {
        Self { name, value }
    }

    pub fn with_sub_value(&self, value_base: String) -> Self {
        Self {
            name: self.name.clone(),
            value: join_path(
                self.value.sub_path(
                    value_base.path()
                )
            )
        }
    }
}

impl Into<u64> for Param {
    fn into(self) -> u64 {
        self.value.parse().unwrap_or(0)
    }
}

impl Into<String> for Param {
    fn into(self) -> String {
        self.value
    }
}

impl Into<bool> for Param {
    fn into(self) -> bool {
        matches!(self.value.to_lowercase().as_str(), "1" | "true" | "yes" | "on")
    }
}

impl Into<f64> for Param {
    fn into(self) -> f64 {
        self.value.parse().unwrap_or(0.0)
    }
}


pub trait Params {
    #[allow(dead_code)]
    fn get_param(&self, name: &str) -> Option<Param>;
    fn get_params(&self) -> Vec<Param>;
    #[allow(dead_code)]
    fn get_param_as<T: std::str::FromStr>(&self, name: &str) -> Option<T> {
        self.get_param(name)
            .and_then(|p| p.value.parse::<T>().ok())
    }
    fn merge_params(&self, params: Vec<Param>) -> Vec<Param> {
        let mut merged = self.get_params();
        for param in params {
            if let Some(existing) = merged.iter_mut().find(|p| p.name == param.name) {
                existing.value = param.value;
            } else {
                merged.push(param);
            }
        }
        merged
    }
}

impl Params for Vec<Param> {
    fn get_params(&self) -> Vec<Param> {
        self.clone()
    }
    fn get_param(&self, name: &str) -> Option<Param> {
        self.iter().find(|p| p.name == name).cloned()
    }
}

pub trait PathString {
    fn path(&self) -> Vec<String>;
    fn sub_path(&self, base_path: Vec<String>) -> Vec<String> {
        let self_path = self.path();
        if self_path.starts_with(&base_path) {
            self_path[base_path.len()..].to_vec()
        } else {
            self_path
        }
    }
}

pub fn join_path(vector : Vec<String>) -> String {
    vector.join(&PATH_SEPARATOR.to_string())
}

impl PathString for String {
    fn path(&self) -> Vec<String> {
        self.trim().split(PATH_SEPARATOR).map(|s| s.to_string()).collect()
    }
}


#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ActionType {
    Shortcut(String),
    Text(String),
    Line(String),
    Paste(String),
    PasteEnter(String),
    Pause(u64),
    OpenUrl(String),
    Custom(ActionParams),
}


#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum BoardType {
    Static,
    Home,
    Chain(ChainParams),
    Custom(BoardParams),
}



#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ActionParams {
    #[serde(rename = "type")]
    pub action_type: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<Param>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BoardParams {
    #[serde(rename = "type")]
    pub board_type: String,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<Param>,
}

impl Params for BoardParams {
    fn get_params(&self) -> Vec<Param> {
        self.params.clone()
    }
    fn get_param(&self, name: &str) -> Option<Param> {
        self.params.iter().find(|p| p.name == name).cloned()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChainParams {
    pub boards: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_board: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<Param>,
}

impl ChainParams {
    pub fn boards(&self) -> Vec<String> {
        self.boards.split(',').map(|s| s.trim().to_string()).collect()
    }
}

impl Params for ChainParams {
    fn get_params(&self) -> Vec<Param> {
        vec![
            Param::new("boards".to_string(), self.boards.clone()),
            Param::new("initial_board".to_string(), self.initial_board.clone().unwrap_or_default()),
        ].into_iter().chain(
            self.params.clone().into_iter()
        ).collect()
    }
    fn get_param(&self, name: &str) -> Option<Param> {
        self.params.iter().find(|p| p.name == name).cloned()
    }
}

impl From<Vec<Param>> for ChainParams {
    fn from(params: Vec<Param>) -> Self {
        let boards = params.get_param_as::<String>("boards")
            .unwrap_or_default();
        let initial = params.get_param_as::<String>("initial_board");
        let other_params = params.into_iter()
            .filter(|p| p.name != "boards" && p.name != "initial_board")
            .collect::<Vec<Param>>();
        Self {
            boards,
            initial_board: initial,
            params: other_params,
        }
    }
}

 #[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_param_into() {
        let p = super::Param::new("key".to_string(), "42".to_string());
        let v: u64 = p.clone().into();
        assert_eq!(v, 42);

        let p = super::Param::new("key".to_string(), "true".to_string());
        let v: bool = p.clone().into();
        assert_eq!(v, true);

        let p = super::Param::new("key".to_string(), "3.14".to_string());
        let v: f64 = p.clone().into();
        assert_eq!(v, 3.14);

        let p = super::Param::new("key".to_string(), "hello".to_string());
        let v: String = p.clone().into();
        assert_eq!(v, "hello");
    }

    #[test]
    fn test_get_param_as() {
        let params = vec![
            super::Param::new("key1".to_string(), "42".to_string()),
            super::Param::new("key2".to_string(), "true".to_string()),
            super::Param::new("key3".to_string(), "3.14".to_string()),
            super::Param::new("key4".to_string(), "hello".to_string()),
        ];

        let v: Option<u64> = params.get_param_as("key1");
        assert_eq!(v, Some(42));

        let v: Option<bool> = params.get_param_as("key2");
        assert_eq!(v, Some(true));

        let v: Option<f64> = params.get_param_as("key3");
        assert_eq!(v, Some(3.14));

        let v: Option<String> = params.get_param_as("key4");
        assert_eq!(v, Some("hello".to_string()));

        let v: Option<u64> = params.get_param_as("nonexistent");
        assert_eq!(v, None);
    }

}

