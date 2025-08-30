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
        )
    }).collect()

}

#[allow(dead_code)]
pub fn convert_pad(pad: &core::Pad, pad_id: PadId, repository: &dyn SettingsRepository) -> Pad {
    Pad::new(
        pad_id,
        pad.clone(),
        pad.color_scheme.as_ref().and_then(|name| repository.get_color_scheme(name)).map(|cs| cs.into()),
        pad.text_style.as_ref().and_then(|name| repository.get_text_style(name)).map(|ts| ts.into()),
        vec![], // tags are a model level concern, not applied when converting core to model
    )
}



/// UseCases - specific operations that can be performed on the model/repository
/// All use cases require repository access

pub struct CreateDetectableBoardUseCase<R: SettingsRepository + SettingsRepositoryMut> {
    repository: Rc<R>,
    process_name: String,
    window_title: Option<String>,
    icon: Option<DetectedIcon>
}

impl<R: SettingsRepository + SettingsRepositoryMut> CreateDetectableBoardUseCase<R> {
    pub fn new(repository: Rc<R>, process_name: String) -> Self {
        Self {
            repository,
            process_name,
            window_title: None,
            icon: None
        }
    }

    pub fn with_window_title(mut self, title: Option<String>) -> Self {
        self.window_title = title;
        self
    }

    pub fn with_icon(mut self, icon: Option<DetectedIcon>) -> Self {
        self.icon = icon;
        self
    }

    pub fn board_name(&self) -> String {
        slugify_process_name(&self.process_name)
    }

    pub fn execute(&self) -> Result<core::Board, Box<dyn std::error::Error>> {
        let name = slugify_process_name(&self.process_name);

        if self.repository.get_board(&name).is_ok() {
            return Err("A board for this process already exists".into());
        }

        let detection = Detection::Win32(self.process_name.replace(".exe", "").to_lowercase());

        let icon_name = match &self.icon {
            Some(detected_icon) => detected_icon.finalize().map(|_| detected_icon.final_name()),
            None => None
        };

        let title = self.window_title.clone().or(Some(name.clone()));

        let board = core::Board {
            name: name.clone(),
            title: title,
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


