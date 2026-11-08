use serde::{Deserialize, Serialize};
use zone_core::{SplitDirection, SplitNode, TabId};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LayoutRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl LayoutRect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }
}

pub struct SplitLayoutEngine;

impl SplitLayoutEngine {
    /// Recursively computes client-relative placement rectangles for every tab in a SplitNode tree.
    pub fn compute_layout(node: &SplitNode, bounds: LayoutRect) -> Vec<(TabId, LayoutRect)> {
        let mut results = Vec::new();
        Self::recurse(node, bounds, &mut results);
        results
    }

    fn recurse(node: &SplitNode, bounds: LayoutRect, out: &mut Vec<(TabId, LayoutRect)>) {
        match node {
            SplitNode::Leaf(tab_id) => {
                out.push((*tab_id, bounds));
            }
            SplitNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let clamped_ratio = ratio.clamp(0.05, 0.95);
                match direction {
                    SplitDirection::Horizontal => {
                        let w1 = bounds.width * clamped_ratio;
                        let w2 = bounds.width - w1;
                        let r1 = LayoutRect::new(bounds.x, bounds.y, w1, bounds.height);
                        let r2 = LayoutRect::new(bounds.x + w1, bounds.y, w2, bounds.height);
                        Self::recurse(first, r1, out);
                        Self::recurse(second, r2, out);
                    }
                    SplitDirection::Vertical => {
                        let h1 = bounds.height * clamped_ratio;
                        let h2 = bounds.height - h1;
                        let r1 = LayoutRect::new(bounds.x, bounds.y, bounds.width, h1);
                        let r2 = LayoutRect::new(bounds.x, bounds.y + h1, bounds.width, h2);
                        Self::recurse(first, r1, out);
                        Self::recurse(second, r2, out);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_leaf() {
        let tab = TabId::new();
        let node = SplitNode::Leaf(tab);
        let bounds = LayoutRect::new(0.0, 0.0, 1000.0, 800.0);
        let layout = SplitLayoutEngine::compute_layout(&node, bounds);
        assert_eq!(layout.len(), 1);
        assert_eq!(layout[0].0, tab);
        assert_eq!(layout[0].1, bounds);
    }

    #[test]
    fn test_2way_horizontal_split() {
        let tab1 = TabId::new();
        let tab2 = TabId::new();
        let node = SplitNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(SplitNode::Leaf(tab1)),
            second: Box::new(SplitNode::Leaf(tab2)),
        };

        let bounds = LayoutRect::new(200.0, 0.0, 800.0, 600.0);
        let layout = SplitLayoutEngine::compute_layout(&node, bounds);
        assert_eq!(layout.len(), 2);
        assert_eq!(layout[0].1, LayoutRect::new(200.0, 0.0, 400.0, 600.0));
        assert_eq!(layout[1].1, LayoutRect::new(600.0, 0.0, 400.0, 600.0));
    }

    #[test]
    fn test_4way_quadrant_split() {
        let t1 = TabId::new();
        let t2 = TabId::new();
        let t3 = TabId::new();
        let t4 = TabId::new();

        let top_row = SplitNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(SplitNode::Leaf(t1)),
            second: Box::new(SplitNode::Leaf(t2)),
        };

        let bottom_row = SplitNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(SplitNode::Leaf(t3)),
            second: Box::new(SplitNode::Leaf(t4)),
        };

        let grid = SplitNode::Split {
            direction: SplitDirection::Vertical,
            ratio: 0.5,
            first: Box::new(top_row),
            second: Box::new(bottom_row),
        };

        let bounds = LayoutRect::new(0.0, 0.0, 1000.0, 800.0);
        let layout = SplitLayoutEngine::compute_layout(&grid, bounds);
        assert_eq!(layout.len(), 4);
        // Top-left
        assert_eq!(layout[0].1, LayoutRect::new(0.0, 0.0, 500.0, 400.0));
        // Top-right
        assert_eq!(layout[1].1, LayoutRect::new(500.0, 0.0, 500.0, 400.0));
        // Bottom-left
        assert_eq!(layout[2].1, LayoutRect::new(0.0, 400.0, 500.0, 400.0));
        // Bottom-right
        assert_eq!(layout[3].1, LayoutRect::new(500.0, 400.0, 500.0, 400.0));
    }
}
