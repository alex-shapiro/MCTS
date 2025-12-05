use crate::game::{Action, Game, GameResult, Player};

pub struct Mcts<G> {
    nodes: Vec<Node<G>>,
    iters: u32,
}

impl<G: Game> Mcts<G> {
    pub fn new(iters: u32) -> Self {
        Self {
            nodes: vec![],
            iters,
        }
    }

    pub fn search(&mut self, state: &G) -> Option<Action> {
        self.nodes.clear();
        self.nodes.push(Node::new(state.clone(), None, None));
        for _ in 0..self.iters {
            let initial_reward = state.current_reward();
            let node_idx = self.select();
            let node_idx = self.expand(node_idx);
            let game_result = self.simulate(node_idx);
            self.backup(node_idx, game_result, initial_reward);
        }
        self.best_action()
    }

    /// Walk the tree to find the first node that is either terminal or has unvisited actions.
    /// If a given node is neither, walk to the child with highest UCB1 score.
    fn select(&self) -> usize {
        let mut idx = 0;

        loop {
            let node = &self.nodes[idx];

            if node.is_terminal() || node.has_unvisited_actions() {
                return idx;
            }

            idx = self.best_child(idx);
        }
    }

    /// Expand a nonterminal node with unvisited actions.
    /// If the node is terminal or has no unvisited actions, return the node itself.
    fn expand(&mut self, node_idx: usize) -> usize {
        let node = &mut self.nodes[node_idx];

        if node.is_terminal() {
            return node_idx;
        }

        let Some(action) = node.unvisited_actions.pop() else {
            return node_idx;
        };

        let mut state = node.state.clone();
        state.step(action).unwrap();
        let child_node = Node::new(state, Some(action), Some(node_idx));
        let child_idx = self.nodes.len();
        self.nodes.push(child_node);
        self.nodes[node_idx].children.push(child_idx);
        child_idx
    }

    /// Simulate the rest of the game with random actions
    fn simulate(&self, node_idx: usize) -> GameResult {
        let mut game = self.nodes[node_idx].state.clone();
        loop {
            if let Some(game_result) = game.result() {
                return game_result;
            }
            let actions = game.allowed_actions();
            let action = actions[fastrand::usize(0..actions.len())];
            game.step(action).unwrap();
        }
    }

    /// Back up visits & rewards
    fn backup(&mut self, node_idx: usize, game_result: GameResult, initial_reward: f64) {
        let mut current = Some(node_idx);
        while let Some(idx) = current {
            let node = &mut self.nodes[idx];
            node.visits += 1.0;
            node.reward += match game_result {
                GameResult::Win(player) => f64::from(player == node.actor()),
                GameResult::Draw => 0.5,
                GameResult::End(reward) => reward as f64 - initial_reward,
            };
            current = node.parent;
        }
    }

    /// Select the "best" action by finding the root node child with the most visits.
    /// As the number of MCTS iterations increases, this value approaches the optimal decision.
    fn best_action(&self) -> Option<Action> {
        self.nodes[0]
            .children
            .iter()
            .map(|idx| &self.nodes[*idx])
            .max_by(|a, b| a.visits.partial_cmp(&b.visits).unwrap())
            .unwrap()
            .action
    }

    /// Select the child node with the highest UCB1 score
    fn best_child(&self, idx: usize) -> usize {
        let node = &self.nodes[idx];
        let visits = node.visits;
        node.children
            .iter()
            .map(|idx| (*idx, self.nodes[*idx].ucb1(visits)))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap()
            .0
    }
}

struct Node<G> {
    state: G,
    action: Option<Action>,
    parent: Option<usize>,
    children: Vec<usize>,
    visits: f64,
    reward: f64,
    unvisited_actions: Vec<Action>,
}

impl<G: Game> Node<G> {
    fn new(state: G, action: Option<Action>, parent: Option<usize>) -> Self {
        let unvisited_actions = state.allowed_actions();
        Node {
            state,
            action,
            parent,
            children: vec![],
            visits: 0.0,
            reward: 0.0,
            unvisited_actions,
        }
    }

    /// Player responsible for the node action
    fn actor(&self) -> Player {
        self.state.current_player().opponent()
    }

    fn is_terminal(&self) -> bool {
        self.state.result().is_some()
    }

    fn has_unvisited_actions(&self) -> bool {
        !self.unvisited_actions.is_empty()
    }

    fn ucb1(&self, parent_visits: f64) -> f64 {
        let r_exploit = self.reward / self.visits;
        let r_explore = (2.0 * parent_visits.ln() / self.visits).sqrt();
        r_exploit + r_explore
    }
}
