use super::{ColorScheme, TextStyle, ModifierState, PadId, Pad, Tag};

pub trait Board {
    #[allow(dead_code)]
    fn name(&self) -> String;
    fn title(&self) -> String;

    fn icon(&self) -> Option<String> { // Icon name or path
        None
    }
    fn color_scheme(&self) -> ColorScheme {
        ColorScheme::default()
    }
    fn text_style(&self) -> TextStyle {
        TextStyle::default()
    }

    fn padset(&self, _modifier: Option<ModifierState>) -> Box<dyn PadSet> {
        Box::new(vec![] as Vec<Pad>)
    }

    fn tags(&self) -> Vec<Tag> {
        vec![]
    }

}

pub trait PadSet {
    fn pads(&self) -> Vec<Pad>;
    fn pad(&self, id: PadId) -> Pad;
    fn update(&mut self, pad: Pad);
    fn flatten(&self) -> Vec<Pad>;
    fn overlay(&self, pads: Vec<Pad>) -> Vec<Pad> {
        let mut result = self.pads();
        for pad in pads {
            if let Some(existing) = result.iter_mut().find(|p| p.pad_id() == pad.pad_id()) {
                *existing = pad;
            } else {
                result.push(pad);
            }
        }
        result
    }
}


impl PadSet for Vec<Pad> {
    fn pads(&self) -> Vec<Pad> {
        self.clone()
    }

    fn pad(&self, id: PadId) -> Pad {
        self.iter()
            .find(|p| p.pad_id() == id)
            .cloned()
            .unwrap_or_else(|| Pad::from(id))
    }

    fn update(&mut self, pad: Pad) {
        if let Some(existing) = self.iter_mut().find(|p| p.pad_id() == pad.pad_id()) {
            *existing = pad;
        } else {
            self.push(pad);
        }
    }
    fn flatten(&self) -> Vec<Pad> {
        let max_id = self.iter().map(|p| p.pad_id()).max().unwrap_or(PadId::One);
        max_id.up_to().iter().map(|id| self.pad(*id)).collect()
    }
}

impl PadSet for PadId {
    fn pads(&self) -> Vec<Pad> {
        vec![Pad::from(*self)]
    }

    fn pad(&self, id: PadId) -> Pad {
        Pad::from(id)
    }

    fn update(&mut self, _pad: Pad) {
        // No-op for single PadId
    }

    fn flatten(&self) -> Vec<Pad> {
        self.pads()
    }
}

impl PadSet for Vec<PadId> {
    fn pads(&self) -> Vec<Pad> {
        let max_id = self.iter().max().cloned().unwrap_or(PadId::One);
        max_id.pads()
    }

    fn pad(&self, id: PadId) -> Pad {
        Pad::from(id)
    }
    fn update(&mut self, _pad: Pad) {
        // No-op for Vec<PadId>
    }
    fn flatten(&self) -> Vec<Pad> {
        self.pads()
    }
}
