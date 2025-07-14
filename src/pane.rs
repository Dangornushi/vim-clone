use std::collections::HashMap;
use ratatui::layout::{Direction, Constraint, Layout, Rect};

/// ペインの分割方向を表す
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

/// ペインの分割情報
#[derive(Debug, Clone)]
pub struct Split {
    pub direction: SplitDirection,
    pub ratio: f64, // 0.0 to 1.0
}

/// 個別のペインを表す構造体
#[derive(Debug, Clone)]
pub struct Pane {
    pub id: usize,
    pub window_index: usize,
    pub rect: Option<Rect>, // 描画時に計算される領域
    pub split: Option<Split>,
    pub children: Vec<usize>, // 子ペインのID
    pub parent: Option<usize>, // 親ペインのID
}

impl Pane {
    pub fn new(id: usize, window_index: usize) -> Self {
        Self {
            id,
            window_index,
            rect: None,
            split: None,
            children: Vec::new(),
            parent: None,
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}

/// ペインの管理とレイアウト計算を行う構造体
#[derive(Debug)]
pub struct PaneManager {
    panes: HashMap<usize, Pane>,
    root_pane: usize,
    active_pane: usize,
    next_id: usize,
}

impl PaneManager {
    pub fn new(initial_window_index: usize) -> Self {
        let mut panes = HashMap::new();
        let root_pane = Pane::new(0, initial_window_index);
        panes.insert(0, root_pane);

        Self {
            panes,
            root_pane: 0,
            active_pane: 0,
            next_id: 1,
        }
    }

    /// 新しいペインIDを生成
    fn next_pane_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// アクティブペインを取得
    pub fn get_active_pane(&self) -> Option<&Pane> {
        self.panes.get(&self.active_pane)
    }

    /// アクティブペインを変更
    pub fn set_active_pane(&mut self, pane_id: usize) {
        if self.panes.contains_key(&pane_id) {
            self.active_pane = pane_id;
        }
    }

    /// ペインを取得
    #[allow(dead_code)]
    pub fn get_pane(&self, pane_id: usize) -> Option<&Pane> {
        self.panes.get(&pane_id)
    }

    /// ペインを変更可能な参照として取得
    pub fn get_pane_mut(&mut self, pane_id: usize) -> Option<&mut Pane> {
        self.panes.get_mut(&pane_id)
    }

    /// 垂直分割（左右に分割）
    pub fn vsplit(&mut self, target_pane_id: usize, new_window_index: usize) -> Option<usize> {
        self.split_pane(target_pane_id, new_window_index, SplitDirection::Horizontal, 0.5)
    }

    /// 水平分割（上下に分割）
    pub fn hsplit(&mut self, target_pane_id: usize, new_window_index: usize) -> Option<usize> {
        self.split_pane(target_pane_id, new_window_index, SplitDirection::Vertical, 0.5)
    }

    /// ペインを分割する内部実装
    fn split_pane(
        &mut self,
        target_pane_id: usize,
        new_window_index: usize,
        direction: SplitDirection,
        ratio: f64,
    ) -> Option<usize> {
        if !self.panes.contains_key(&target_pane_id) {
            return None;
        }

        let new_pane_id = self.next_pane_id();
        
        // 既存のペインの情報を取得
        let target_window_index = self.panes[&target_pane_id].window_index;
        
        // 新しいペインを作成
        let mut new_pane = Pane::new(new_pane_id, new_window_index);
        new_pane.parent = Some(target_pane_id);

        // 既存のペインも子ペインとして作成
        let existing_child_id = self.next_pane_id();
        let mut existing_child = Pane::new(existing_child_id, target_window_index);
        existing_child.parent = Some(target_pane_id);

        // ターゲットペインを分割設定で更新
        if let Some(target_pane) = self.panes.get_mut(&target_pane_id) {
            target_pane.split = Some(Split { direction, ratio });
            target_pane.children = vec![existing_child_id, new_pane_id];
        }

        // 新しいペインを追加
        self.panes.insert(new_pane_id, new_pane);
        self.panes.insert(existing_child_id, existing_child);

        Some(new_pane_id)
    }

    /// ペインを閉じる
    pub fn close_pane(&mut self, pane_id: usize) -> bool {
        if pane_id == self.root_pane || !self.panes.contains_key(&pane_id) {
            return false; // ルートペインは閉じられない
        }

        // 親ペインを取得
        let parent_id = match self.panes[&pane_id].parent {
            Some(id) => id,
            None => return false,
        };

        // 兄弟ペインを取得
        let siblings: Vec<usize> = self.panes[&parent_id]
            .children
            .iter()
            .filter(|&&id| id != pane_id)
            .copied()
            .collect();

        if siblings.len() != 1 {
            return false; // 兄弟が1つでない場合は処理しない
        }

        let sibling_id = siblings[0];

        // 兄弟の内容を親に移動
        let sibling_pane = self.panes[&sibling_id].clone();
        if let Some(parent_pane) = self.panes.get_mut(&parent_id) {
            parent_pane.window_index = sibling_pane.window_index;
            parent_pane.split = sibling_pane.split;
            parent_pane.children = sibling_pane.children.clone();
        }

        // 兄弟の子ペインの親を更新
        for &child_id in &sibling_pane.children {
            if let Some(child_pane) = self.panes.get_mut(&child_id) {
                child_pane.parent = Some(parent_id);
            }
        }

        // 閉じるペインと兄弟ペインを削除
        self.panes.remove(&pane_id);
        self.panes.remove(&sibling_id);

        // アクティブペインが閉じられた場合、親に変更
        if self.active_pane == pane_id {
            self.active_pane = parent_id;
        }

        true
    }

    /// レイアウトを計算してペインの描画領域を設定
    pub fn calculate_layout(&mut self, area: Rect) {
        self.calculate_pane_layout(self.root_pane, area);
    }

    /// 再帰的にペインのレイアウトを計算
    fn calculate_pane_layout(&mut self, pane_id: usize, area: Rect) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            pane.rect = Some(area);

            if !pane.children.is_empty() {
                if let Some(split) = &pane.split {
                    let children = pane.children.clone();
                    match split.direction {
                        SplitDirection::Horizontal => {
                            let chunks = Layout::default()
                                .direction(Direction::Horizontal)
                                .constraints([
                                    Constraint::Percentage((split.ratio * 100.0) as u16),
                                    Constraint::Percentage(((1.0 - split.ratio) * 100.0) as u16),
                                ])
                                .split(area);
                            
                            for (i, &child_id) in children.iter().enumerate() {
                                if i < chunks.len() {
                                    self.calculate_pane_layout(child_id, chunks[i]);
                                }
                            }
                        }
                        SplitDirection::Vertical => {
                            let chunks = Layout::default()
                                .direction(Direction::Vertical)
                                .constraints([
                                    Constraint::Percentage((split.ratio * 100.0) as u16),
                                    Constraint::Percentage(((1.0 - split.ratio) * 100.0) as u16),
                                ])
                                .split(area);
                            
                            for (i, &child_id) in children.iter().enumerate() {
                                if i < chunks.len() {
                                    self.calculate_pane_layout(child_id, chunks[i]);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// すべてのリーフペイン（実際に描画されるペイン）を取得
    pub fn get_leaf_panes(&self) -> Vec<&Pane> {
        self.panes
            .values()
            .filter(|pane| pane.is_leaf())
            .collect()
    }

    /// ペインナビゲーション: 左のペインに移動
    pub fn move_to_left_pane(&mut self) {
        if let Some(target_id) = self.find_adjacent_pane(SplitDirection::Horizontal, false) {
            self.active_pane = target_id;
        }
    }

    /// ペインナビゲーション: 右のペインに移動
    pub fn move_to_right_pane(&mut self) {
        if let Some(target_id) = self.find_adjacent_pane(SplitDirection::Horizontal, true) {
            self.active_pane = target_id;
        }
    }

    /// ペインナビゲーション: 上のペインに移動
    pub fn move_to_up_pane(&mut self) {
        if let Some(target_id) = self.find_adjacent_pane(SplitDirection::Vertical, false) {
            self.active_pane = target_id;
        }
    }

    /// ペインナビゲーション: 下のペインに移動
    pub fn move_to_down_pane(&mut self) {
        if let Some(target_id) = self.find_adjacent_pane(SplitDirection::Vertical, true) {
            self.active_pane = target_id;
        }
    }

    /// 隣接するペインを見つける
    fn find_adjacent_pane(&self, direction: SplitDirection, forward: bool) -> Option<usize> {

        let leaf_panes = self.get_leaf_panes();
        let current_pane = self.get_active_pane()?;
        let current_rect = current_pane.rect?;

        let mut best_candidate: Option<(usize, u16)> = None;

        for pane in leaf_panes {
            if pane.id == current_pane.id {
                continue;
            }
            let pane_rect = pane.rect?;

            let is_adjacent = match direction {
                SplitDirection::Horizontal => { // Left/Right
                    let y_overlap = current_rect.y < pane_rect.bottom() && pane_rect.y < current_rect.bottom();
                    if !y_overlap { continue; }
                    if forward { // Right
                        pane_rect.x >= current_rect.right()
                    } else { // Left
                        pane_rect.right() <= current_rect.x
                    }
                }
                SplitDirection::Vertical => { // Up/Down
                    let x_overlap = current_rect.x < pane_rect.right() && pane_rect.x < current_rect.right();
                    if !x_overlap { continue; }
                    if forward { // Down
                        pane_rect.y >= current_rect.bottom()
                    } else { // Up
                        pane_rect.bottom() <= current_rect.y
                    }
                }
            };

            if is_adjacent {
                let distance = match direction {
                    SplitDirection::Horizontal => {
                        if forward { pane_rect.x.saturating_sub(current_rect.right()) }
                        else { current_rect.x.saturating_sub(pane_rect.right()) }
                    }
                    SplitDirection::Vertical => {
                        if forward { pane_rect.y.saturating_sub(current_rect.bottom()) }
                        else { current_rect.y.saturating_sub(pane_rect.bottom()) }
                    }
                };

                if best_candidate.is_none() || distance < best_candidate.unwrap().1 {
                    best_candidate = Some((pane.id, distance));
                }
            }
        }
        best_candidate.map(|(id, _)| id)
    }

    /// アクティブペインIDを取得
    pub fn get_active_pane_id(&self) -> usize {
        self.active_pane
    }

    /// ルートペインIDを取得
    #[allow(dead_code)]
    pub fn get_root_pane_id(&self) -> usize {
        self.root_pane
    }
}