use crate::core::integration::ChainParams;
use crate::core::{self, slugify_process_name, BoardType, DetectedIcon, Detection, SettingsRepository, SettingsRepositoryMut};
use crate::model::{ColorScheme, ModifierState, Pad, PadId, PadSet, TextStyle};
use std::rc::Rc;

pub struct BoardHandle<R: SettingsRepository> {
    repository: Rc<R>,
    board_name: String,
}

pub struct PadSetHandle<R: SettingsRepository> {
    repository: Rc<R>,
    padset_name: String,
}

impl<R: SettingsRepository> BoardHandle<R> {
    pub fn new(repository: Rc<R>, board_name: String) -> Self {
        Self {
            repository,
            board_name,
        }
    }

    pub fn name(&self) -> &str {
        &self.board_name
    }

    pub fn title(&self) -> Result<String, Box<dyn std::error::Error>> {
        let board = self.repository.get_board(&self.board_name)?;
        Ok(board.title().to_string())
    }

    pub fn icon(&self) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let board = self.repository.get_board(&self.board_name)?;
        Ok(if board.icon().is_empty() { None } else { Some(board.icon().to_string()) })
    }

    pub fn color_scheme(&self) -> Result<ColorScheme, Box<dyn std::error::Error>> {
        let board = self.repository.get_board(&self.board_name)?;
        Ok(self.repository.resolve_color_scheme(&board.color_scheme))
    }

    pub fn text_style(&self) -> Result<TextStyle, Box<dyn std::error::Error>> {
        let board = self.repository.get_board(&self.board_name)?;
        Ok(self.repository.resolve_text_style(&board.text_style))
    }

    pub fn padset(&self, modifier: Option<ModifierState>) -> Result<PadSetHandle<R>, Box<dyn std::error::Error>> {
        let board = self.repository.get_board(&self.board_name)?;
        Ok(PadSetHandle::new(
            self.repository.clone(),
            board
                .padset_name(modifier.map(|m| m.to_string()).as_deref())
                .ok_or("PadSet not found for the given modifier")?
                .into(),
        ))
    }

}

impl<R: SettingsRepository + SettingsRepositoryMut> BoardHandle<R> {
    pub fn set_title(&self, title: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
        let mut board = self.repository.get_board(&self.board_name)?;
        board.title = title;
        self.repository.set_board(board)?;
        Ok(())
    }

    pub fn set_color_scheme(&self, color_scheme: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
        let mut board = self.repository.get_board(&self.board_name)?;

        board.color_scheme = color_scheme
            .as_ref()
            .and_then(|name| self.repository.get_color_scheme(name))
            .map(|cs| cs.name.clone());

        self.repository.set_board(board)?;
        Ok(())
    }

    pub fn set_text_style(&self, text_style: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
        let mut board = self.repository.get_board(&self.board_name)?;

        board.text_style = text_style
            .as_ref()
            .and_then(|name| self.repository.get_text_style(name))
            .map(|ts| ts.name.clone());

        self.repository.set_board(board)?;
        Ok(())
    }
}

#[allow(dead_code)]
impl<R: SettingsRepository> PadSetHandle<R> {
    pub fn new(repository: Rc<R>, padset_name: String) -> Self {
        Self {
            repository,
            padset_name,
        }
    }

    pub fn name(&self) -> &str {
        &self.padset_name
    }

    pub fn pads(&self) -> Result<Vec<Pad>, Box<dyn std::error::Error>> {
        let padset = self.repository.get_padset(&self.padset_name)?;
        Ok(convert_padset(&padset.items, self.repository.as_ref()))
    }

}

impl<R: SettingsRepository + SettingsRepositoryMut> PadSetHandle<R> {
    pub fn set_pad(&self, pad: Pad) -> Result<(), Box<dyn std::error::Error>> {
        let pads = self.pads()?.overlay(vec![pad]).flatten().iter().map(|p| p.as_data()).collect();
        self.repository.set_padset(core::PadSet::new(&self.padset_name, pads))?;
        Ok(())
    }
}

pub struct ColorSchemeHandle<R: SettingsRepository> {
    repository: Rc<R>,
    name: String,
}

impl<R: SettingsRepository> ColorSchemeHandle<R> {
    pub fn new(repository: Rc<R>, name: Option<String>) -> Self {
        Self { repository, name: name.unwrap_or_else(|| ColorScheme::default().name) }
    }


    pub fn as_data(&self) -> Result<ColorScheme, ()> {
        self.repository.get_color_scheme(&self.name)
            .ok_or(())
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}



impl<R: SettingsRepository> ColorSchemeHandle<R> {
    pub fn next_name(&self) -> Option<String> {
        let schemes = self.repository.color_schemes();
        if let Some(pos) = schemes.iter().position(|s| s == &self.name) {
            let next_pos = (pos + 1) % schemes.len();
            Some(schemes[next_pos].clone())
        } else {
            None
        }
    }
    pub fn prev_name(&self) -> Option<String> {
        let schemes = self.repository.color_schemes();
        if let Some(pos) = schemes.iter().position(|s| s == &self.name) {
            let prev_pos = if pos == 0 { schemes.len() - 1 } else { pos - 1 };
            Some(schemes[prev_pos].clone())
        } else {
            None
        }
    }

    pub fn move_next(&mut self) {
        self.next_name().map(|next| {
            self.name = next;
        });
    }

    pub fn move_prev(&mut self) {
        self.prev_name().map(|prev| {
            self.name = prev;
        });
    }

    pub fn select(&mut self, name: String) {
        self.name = name;
    }
}


impl<R: SettingsRepository + SettingsRepositoryMut> ColorSchemeHandle<R> {
    #[allow(dead_code)]
    pub fn update(&self, new_scheme: ColorScheme) -> Result<(), Box<dyn std::error::Error>> {
        if new_scheme.name != self.name {
            return Err("Cannot change the name of an existing ColorScheme".into());
        }
        self.repository.set_color_scheme(new_scheme)?;
        Ok(())
    }
}



pub struct TextStyleHandle<R: SettingsRepository> {
    repository: Rc<R>,
    name: String,
}

impl<R: SettingsRepository> TextStyleHandle<R> {
    pub fn new(repository: Rc<R>, name: Option<String>) -> Self {
        Self { repository, name: name.unwrap_or_else(|| TextStyle::default().name) }
    }

    pub fn as_data(&self) -> Result<TextStyle, ()> {
        self.repository.get_text_style(&self.name)
            .ok_or(())
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl<R: SettingsRepository> TextStyleHandle<R> {
    pub fn next_name(&self) -> Option<String> {
        let styles = self.repository.text_styles();
        if let Some(pos) = styles.iter().position(|s| s == &self.name) {
            let next_pos = (pos + 1) % styles.len();
            Some(styles[next_pos].clone())
        } else {
            None
        }
    }
    pub fn prev_name(&self) -> Option<String> {
        let styles = self.repository.text_styles();
        if let Some(pos) = styles.iter().position(|s| s == &self.name) {
            let prev_pos = if pos == 0 { styles.len() - 1 } else { pos - 1 };
            Some(styles[prev_pos].clone())
        } else {
            None
        }
    }

    pub fn move_next(&mut self) {
        self.next_name().map(|next| {
            self.name = next;
        });
    }

    pub fn move_prev(&mut self) {
        self.prev_name().map(|prev| {
            self.name = prev;
        });
    }

    pub fn select(&mut self, name: String) {
        self.name = name;
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut> TextStyleHandle<R> {
    #[allow(dead_code)]
    pub fn update(&self, new_style: TextStyle) -> Result<(), Box<dyn std::error::Error>> {
        if new_style.name != self.name {
            return Err("Cannot change the name of an existing TextStyle".into());
        }
        self.repository.set_text_style(new_style)?;
        Ok(())
    }
}



pub fn convert_padset(pads: &[core::Pad], repository: &dyn SettingsRepository) -> Vec<Pad> {
    let all_pad_ids: Vec<PadId> = PadId::all();
    // create one output pad for each input pad, assigninhg pad IDs in order
    pads.iter().enumerate().map(|(i, p)| {
        let pad_id = all_pad_ids.get(i).cloned().unwrap();
        Pad::new(
            pad_id,
            p.clone(),
            p.color_scheme.as_ref().and_then(|name| repository.get_color_scheme(name)).map(|cs| cs.into()),
            p.text_style.as_ref().and_then(|name| repository.get_text_style(name)).map(|ts| ts.into()),
            vec![], // tags are a model level concern, not applied when converting core to model
            false
        )
    }).collect()

}

/// UseCases - require repository access

pub struct CreateDetectableBoardUseCase<R: SettingsRepository + SettingsRepositoryMut> {
    repository: Rc<R>,
    process_name: String,
    #[allow(dead_code)]
    window_title: Option<String>,
    icon: Option<DetectedIcon>
}

impl<R: SettingsRepository + SettingsRepositoryMut> CreateDetectableBoardUseCase<R> {
    pub fn new(repository: Rc<R>, process_name: String, window_title: Option<String>, icon: Option<DetectedIcon>) -> Self {
        Self {
            repository,
            process_name,
            window_title,
            icon
        }
    }

    pub fn board_name(&self) -> String {
        slugify_process_name(&self.process_name)
    }

    pub fn create_board(&self) -> Result<core::Board, Box<dyn std::error::Error>> {
        let name = slugify_process_name(&self.process_name);

        if self.repository.get_board(&name).is_ok() {
            return Err("A board for this process already exists".into());
        }

        let detection = Detection::Win32(self.process_name.replace(".exe", "").to_lowercase());

        let icon_name = match &self.icon {
            Some(detected_icon) => detected_icon.finalize().map(|_| detected_icon.final_name()),
            None => None
        };

        // let title = self.window_title.clone().or(Some(name.clone()));

        let board = core::Board {
            name: name.clone(),
            title: name.clone().into(),
            icon: icon_name,
            board_type: BoardType::Static,
            color_scheme: None,
            text_style: None,
            detection: detection,
            base_pads: Some(name.clone()),
            modifier_pads: Default::default(),
        };

        let padset = core::PadSet::new(name.as_str(), vec![]);

        self.repository.add_board(board)?;
        self.repository.add_padset(padset)?;
        self.repository.get_board(&name)
    }
}

/// Creates a new non-detectable Static board with visual settings copied from the parent board.
/// Name of the new board is formed by appending a slugified version of the keyword to the parent board's name.
/// e.g if parent_board is "chrome" and keyword is "console", new board name will be "chrome/console".
pub fn create_board<R: SettingsRepository + SettingsRepositoryMut>(
    repository: &R, parent_board: String, keyword: String
) -> Result<core::Board, Box<dyn std::error::Error>> {

    let root_parent = if parent_board.contains('/') {
        let (base, _) = parent_board.rsplit_once('/').unwrap();
        if let Ok(_) = repository.get_board(base) {
            base.to_string()
        } else {
            parent_board.clone()
        }
    } else {
        parent_board.clone()
    };

    let name = format!("{}/{}", root_parent, slugify_process_name(&keyword));

    // ensure board does not already exist
    if repository.get_board(&name).is_ok() {
        return Err("A board with this name already exists".into());
    }

    // ensure parent board exists and is a detectable static board with Win32 detection
    let parent = repository.get_board(&parent_board)
        .map_err(|_| "Parent board does not exist")?;

    match parent.board_type {
        BoardType::Static => {}
        _ => return Err("Parent board must be of type Static".into()),
    }

    // match parent.detection {
    //     Detection::Win32(_) => {}
    //     _ => return Err("Parent board must have Win32 detection".into()),
    // }

    // create the new board, inheriting color scheme, text style and icon from parent, setting title to keyword
    let board = core::Board {
        name: name.clone(),
        title: Some(keyword.clone()),
        icon: parent.icon.clone(),
        color_scheme: parent.color_scheme.clone(),
        text_style: parent.text_style.clone(),
        base_pads: Some(name.clone()),
        ..Default::default()
    };

    let padset = core::PadSet::new(name.as_str(), vec![]);

    repository.add_board(board)?;
    repository.add_padset(padset)?;
    repository.get_board(&name)
}



pub struct DeleteBoardUseCase<R: SettingsRepository + SettingsRepositoryMut> {
    repository: Rc<R>,
    board_name: String
}

impl<R: SettingsRepository + SettingsRepositoryMut> DeleteBoardUseCase<R> {
    pub fn new(repository: Rc<R>, board_name: String) -> Self {
        Self {
            repository,
            board_name,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        // Check for references in PadSets (pad.board - board navigation)
        let first_referencing_padset = self.repository.padsets().iter()
            .find_map(|padset_name| {
                if let Ok(padset) = self.repository.get_padset(padset_name) {
                    for pad in &padset.items {
                        if pad.board.as_ref().map_or(false, |b| b == &self.board_name) {
                            return Some(padset_name.clone());
                        }
                    }
                }
                None
            });

        match first_referencing_padset {
            Some(ref padset_name) => {
                return Err(format!("Board\n\"{}\"\nis referenced by PadSet\n\"{}\"", self.board_name, padset_name).into());
            },
            _ => {}
        }

        // Check for references in chain boards
        let first_referencing_chain_board = self.repository.boards().iter()
            .find_map(|board_name| {
                if let Ok(board) = self.repository.get_board(board_name) {
                    if let BoardType::Chain(params) = &board.board_type {
                        if params.boards().contains(&self.board_name) {
                            return Some(board_name.clone());
                        }
                    }
                }
                None
            });

        match first_referencing_chain_board {
            Some(ref board_name) => {
                return Err(format!("Board \"{}\"\nis listed in Collection\n\"{}\"", self.board_name, board_name).into());
            },
            _ => {}
        }

        Ok(())
    }

    pub fn delete(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.validate()?;
        delete_board(self.repository.as_ref(), self.board_name.clone())
    }
}

/// Deletes a board and its associated pad sets.
pub fn delete_board<R: SettingsRepository + SettingsRepositoryMut>(
    repository: &R, board_name: String
) -> Result<(), Box<dyn std::error::Error>> {
    let board = repository.get_board(&board_name)?;

    repository.delete_board(&board.name)?;

    if let Some(base_padset_name) = board.base_pads {
        repository.delete_padset(&base_padset_name)?;
    }

    for padset_name in board.modifier_pads.values() {
        repository.delete_padset(padset_name)?;
    }

    Ok(())
}

pub fn create_modifier_pad_set<R: SettingsRepository + SettingsRepositoryMut>(
    repository: &R, board_name: String, modifier: ModifierState
) -> Result<core::PadSet, Box<dyn std::error::Error>> {
    let mut board = repository.get_board(&board_name)?;
    let modifier = &modifier.to_string();

    if board.modifier_pads.contains_key(modifier) {
        return Err("This modifier pad set already exists for the board".into());
    }

    let padset_name = format!("{}/{}", board_name, slugify_process_name(modifier));
    let padset = core::PadSet::new(padset_name.as_str(), vec![]);

    board.modifier_pads.insert(modifier.to_string(), padset_name.clone());

    repository.add_padset(padset)?;
    repository.set_board(board)?;
    repository.get_padset(&padset_name)
}

pub fn delete_modifier_pad_set<R: SettingsRepository + SettingsRepositoryMut>(
    repository: &R, board_name: String, modifier: String
) -> Result<(), Box<dyn std::error::Error>> {
    let mut board = repository.get_board(&board_name)?;

    if let Some(padset_name) = board.modifier_pads.remove(&modifier) {
        repository.delete_padset(&padset_name)?;
        repository.set_board(board)?;
        Ok(())
    } else {
        Err("This modifier pad set does not exist for the board".into())
    }
}

pub struct ConvertToBoardChainUseCase<R: SettingsRepository + SettingsRepositoryMut> {
    repository: Rc<R>,
    board_name: String,
}

impl<R: SettingsRepository + SettingsRepositoryMut> ConvertToBoardChainUseCase<R> {
    pub fn new(repository: Rc<R>, board_name: String) -> Self {
        Self {
            repository,
            board_name,
        }
    }

    pub fn convert(&self) -> Result<(), Box<dyn std::error::Error>> {
        convert_to_board_chain(self.repository.as_ref(), self.board_name.clone())
    }

    pub fn validate(&self) -> Result<(), String> {
        self.validate_board_not_a_chain(&self.board_name)?;
        self.validate_board_not_in_another_chain(&self.board_name)?;
        Ok(())
    }

    fn validate_board_not_a_chain(&self, board_name: &String) -> Result<(), String> {
        if let Ok(board) = self.repository.get_board(board_name) {
            if matches!(board.board_type, BoardType::Chain(_)) {
                return Err(format!("Board\n\"{}\"\nis already a Collection", board_name));
            }
        }
        Ok(())
    }


    fn validate_board_not_in_another_chain(&self, board_name: &String) -> Result<(), String> {
        for other_board_name in self.repository.boards().iter() {
            if let Ok(other_board) = self.repository.get_board(other_board_name) {
                if let BoardType::Chain(params) = &other_board.board_type {
                    if params.boards().contains(board_name) {
                        return Err(format!("Cannot convert\n\"{}\"\nas it is listed in\n\"{}\"", board_name, other_board.name));
                    }
                }
            }
        }
        Ok(())
    }

}

pub fn convert_to_board_chain<R: SettingsRepository + SettingsRepositoryMut>(
    repository: &R,
    board_name: String
) -> Result<(), Box<dyn std::error::Error>> {

    let mut original_board = repository.get_board(&board_name)?;

    match &original_board.board_type {
        BoardType::Static=> {}
        _ => { return Err("Only Static boards can be converted to BoardChain".into()); }
    }

    let renamed_board_name = format!("{}/stub", board_name);
    repository.rename_board(&original_board.name, &renamed_board_name)?;

    let mut renamed_board = repository.get_board(&renamed_board_name)?;
    renamed_board.detection = Detection::None;

    original_board.base_pads = None;
    original_board.modifier_pads.clear();
    original_board.text_style = None;
    original_board.color_scheme = None;
    original_board.board_type = BoardType::Chain(ChainParams {
        boards: renamed_board_name.clone(),
        initial_board: None,
        params: vec![],
    });

    repository.set_board(renamed_board)?;
    repository.insert_board(&renamed_board_name, original_board)?;

    Ok(())
}

pub fn create_new_chain_with_board<R: SettingsRepository + SettingsRepositoryMut>(
    repository: &R,
    board_name: &String
) -> Result<core::Board, Box<dyn std::error::Error>> {

    let mut board = repository.get_board(&board_name)?;

    match &board.board_type {
        BoardType::Static=> {}
        _ => { return Err("Only Static boards can be added to BoardChain".into()); }
    }

    let chain_params = ChainParams {
        boards: board_name.clone(),
        initial_board: board_name.clone().into(),
        params: vec![],
    };

    board.detection = Detection::None;
    board.name = format!("{}/chain", board_name);
    board.base_pads = None;
    board.modifier_pads.clear();
    board.text_style = None;
    board.color_scheme = None;
    board.board_type = BoardType::Chain(chain_params);

    repository.insert_board(&board_name, board.clone())?;

    return repository.get_board(&board.name)
}