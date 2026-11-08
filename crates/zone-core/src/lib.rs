use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TabId(pub Uuid);

impl TabId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TabId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpaceId(pub Uuid);

impl SpaceId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SpaceId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SplitNode {
    Leaf(TabId),
    Split {
        direction: SplitDirection,
        ratio: f32, // 0.0 .. 1.0 (e.g. 0.50)
        first: Box<SplitNode>,
        second: Box<SplitNode>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl BoundingBox {
    pub fn center(&self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractiveElement {
    pub id: usize,
    pub tag: String,
    pub role: String,
    pub text: String,
    pub placeholder: Option<String>,
    pub bounds: BoundingBox,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineMode {
    Auto,
    PureRust,
    WebView2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomSnapshot {
    pub url: String,
    pub title: String,
    pub engine_used: EngineMode,
    pub elements: Vec<InteractiveElement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum AgentAction {
    Click {
        target_id: usize,
    },
    Input {
        target_id: usize,
        text: String,
        #[serde(default)]
        clear_first: bool,
    },
    Navigate {
        url: String,
    },
    Scroll {
        direction: String,
        #[serde(default = "default_scroll_amount")]
        amount: f32,
    },
    Key {
        key: String,
    },
    Wait {
        #[serde(default = "default_wait_ms")]
        ms: u64,
    },
}

fn default_scroll_amount() -> f32 {
    500.0
}

fn default_wait_ms() -> u64 {
    1000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub success: bool,
    pub message: String,
    pub new_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Space {
    pub id: SpaceId,
    pub name: String,
    pub icon: String,
    pub color_gradient: (String, String),
    pub tabs: Vec<TabId>,
    pub active_tab: Option<TabId>,
}
