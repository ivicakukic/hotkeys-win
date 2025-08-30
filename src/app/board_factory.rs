use std::collections::HashMap;
use std::rc::Rc;

use crate::core::{BoardType, Param, Resources, SettingsRepository, SettingsRepositoryMut, Params};
use crate::components::{ BoardComponent, HomeBoard, MainBoard, SettingsBoard, StateMachineBoard };

pub struct BoardRuntimeContext<R: SettingsRepository + SettingsRepositoryMut> {
    pub repository: Rc<R>,
}

pub trait BoardFactory<R: SettingsRepository + SettingsRepositoryMut> {
    fn create_board(
        &self,
        context: &BoardRuntimeContext<R>,
        board: &crate::core::Board,
        params: Vec<Param>
    ) -> Result<Box<dyn BoardComponent>, Box<dyn std::error::Error>>;
}


pub struct BoardFactoryRegistry<R: SettingsRepository + SettingsRepositoryMut> {
    factories: HashMap<String, Box<dyn BoardFactory<R>>>,
}

impl<R: SettingsRepository + SettingsRepositoryMut> BoardFactoryRegistry<R> {
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn register_factory(&mut self, name: &str, factory: Box<dyn BoardFactory<R>>) {
        self.factories.insert(name.to_string(), factory);
    }

    pub fn get_factory(&self, name: &str) -> Option<&Box<dyn BoardFactory<R>>> {
        self.factories.get(name)
    }
}


pub struct BoardFactoryImpl<'a, R: SettingsRepository + SettingsRepositoryMut> {
    repository: Rc<R>,
    registry: &'a BoardFactoryRegistry<R>,
    resources: Resources,
}

impl<'a, R: SettingsRepository + SettingsRepositoryMut + 'static> BoardFactoryImpl<'a, R> {
    pub fn new(repository: Rc<R>, registry: &'a BoardFactoryRegistry<R>, resources: Resources) -> Self {
        Self { repository, registry, resources }
    }

    pub fn create_board(&self, name: &str, dynamic_params: Vec<Param>) -> Result<Box<dyn BoardComponent>, Box<dyn std::error::Error>> {
        let board = self.repository.get_board(name)?;
        let context = BoardRuntimeContext { repository: self.repository.clone() };
        match &board.board_type {
            BoardType::Static => create_main_board(&context, &board, dynamic_params, self.resources.clone()),
            BoardType::Home => create_home_board(&context, &board, dynamic_params, self.resources.clone()),
            BoardType::Custom(params) => {
                match self.registry.get_factory(&params.board_type) {
                    Some(factory) => factory.create_board(&context, &board, params.merge_params(dynamic_params)),
                    None => Err("Custom board factory not found".into()),
                }
            }
        }
    }
}


fn create_home_board<R: SettingsRepository + SettingsRepositoryMut + 'static>(
    context: &BoardRuntimeContext<R>,
    board: &crate::core::Board,
    params: Vec<Param>,
    resources: Resources
) -> Result<Box<dyn BoardComponent>, Box<dyn std::error::Error>> {

    if board.name == "settings" {
        Ok(
            Box::new(
                StateMachineBoard::new(
                    Box::new(
                        SettingsBoard::new(
                            board.clone(),
                            params,
                            resources,
                            context.repository.clone(),
                        )
                    )
                )
            )
        )
    } else {
        Ok(
            Box::new(
                StateMachineBoard::new(
                    Box::new(
                        HomeBoard::new(
                            board.clone(),
                            params,
                            resources,
                            context.repository.clone(),
                        )
                    )
                )
            )
        )
    }
}



fn create_main_board<R: SettingsRepository + SettingsRepositoryMut + 'static>(
    context: &BoardRuntimeContext<R>,
    board: &crate::core::Board,
    params: Vec<Param>,
    resources: Resources
) -> Result<Box<dyn BoardComponent>, Box<dyn std::error::Error>> {

    Ok(
        Box::new(
            StateMachineBoard::new(
                Box::new(
                    MainBoard::new(
                        board.name.clone(),
                        params,
                        resources,
                        context.repository.clone()
                    )
                )
            )
        )
    )
}