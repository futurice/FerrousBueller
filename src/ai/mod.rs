/** Rust Tyckiting client - A websocket client for a fight to kill all other bots
 *  Copyright Futurice Oy (2015)
 *
 *  This file is part of Rust Tyckiting client.
 *
 *  Rust Tyckiting client is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Rust Tyckiting client is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Rust Tyckiting client.  If not, see <http://www.gnu.org/licenses/>.
 */
extern crate rand;

use super::incoming::{Event, Team, TeamNoPosNoHp, Bot};
use super::{Position, GameConfig};

use self::rand::{thread_rng, Rng};
use std::default::Default;

pub trait Ai {
    fn respond(&mut self, Vec<Event>) -> Vec<Action>;
    fn set_state(&mut self, config: GameConfig, you: Team, other_teamss: Vec<TeamNoPosNoHp>) -> ();
    fn get_bot_by_id(&mut self, bot_id:u32) -> Option<&Bot>;
    fn get_bot_role(&mut self, bot_id:u32) -> Option<&BotRole>;
    fn set_bot_role(&mut self, bot_id:u32, new_state: BotRole);
    fn is_on_playing_field(&self, pos: &Position) -> bool;
}

#[derive(Debug, Default, Clone)]
struct MapTile {
    pos: Position,
    asteroid: bool
}

impl PartialEq for MapTile {
    fn eq(&self, other: &MapTile) -> bool {
        self.pos == other.pos
    }
}

#[derive(Debug, Default)]
struct State {
    //global state
    last_target: Option<Position>,
    asteroid_map: Vec<MapTile>,
    //bot state
    bot_states: Vec<BotState>,
    shoot_count: i32
}

#[derive(Debug)]
enum BotRole{
    SearchMoving(bool),
    Dodging(bool),
    Radaring(bool),
    Shooting(bool),
}

#[derive(Debug)]
struct BotState{
    bot_id: u32,
    current_role: BotRole
}

#[derive(Default)]
struct RandomAi {
    config: GameConfig,
    you: Team,
    other_teams: Vec<TeamNoPosNoHp>,
    current_state: State
}

fn filter_asteroids(positions: Vec<Position>, asteroids: Vec<MapTile>) -> Vec<Position> {
    positions.into_iter().filter(|pos| !asteroids.contains(&MapTile { pos: *pos, asteroid: true })).collect()
}

impl Ai for RandomAi {

    #[allow(unused_variables, unused_assignments)]
    fn respond(&mut self, events: Vec<Event>) -> Vec<Action>  {

        let mut acquired_target: Option<Position> = None;
        let mut move_next = false;
        let mut spotter_bot_id: Option<u32> = None;
        let mut shoot_count = self.current_state.shoot_count;

        for event in events.into_iter()
        {
            match event {
                Event::DamagedEvent(de) => {
                    //println!("Bot ID: {} dealt {} damage!",de.bot_id, de.damage);
                    move_next = true;
                },
                Event::HitEvent(he) => { //hit another ship
                     acquired_target = match self.current_state.last_target{
                         Some(ref tar) => Some(Position {x: tar.x, y:tar.y}),
                         None => None
                     };
                     //println!("Bot ID: {} hit enemy bot: {}", he.source, he.bot_id);
                },
                Event::DieEvent(de) => println!("Bot ID: {} died.", de.bot_id),
                Event::SeeEvent(se) =>{
                    spotter_bot_id = Some(se.source);
                    acquired_target = Some(Position{x: se.pos.x, y: se.pos.y});
                    shoot_count = 0;
                    match acquired_target{
                        Some(ref tar) => {
                            self.current_state.last_target = Some(Position{x: tar.x, y:tar.y})
                        },
                        None => {}
                    };
                    //println!("Bot ID: {} saw bot: {} at x:{}, y:{}", se.bot_id,
                    //se.source, se.pos.x, se.pos.y)
                },
                Event::RadarEchoEvent(ree) => {
                    acquired_target = Some(Position { x: ree.pos.x, y: ree.pos.y });
                    match acquired_target{
                        Some(ref tar) => self.current_state.last_target = Some(Position {x:tar.x, y:tar.y}),
                        None => {}
                    };
                    //println!("An enemy was radar-detected in a radius centered at x:{}, y:{}",
                    //    ree.pos.x, ree.pos.y)
                },
                Event::DetectedEvent(dte) => {
                    //println!("Bot ID: {} got radar-detected!", dte.bot_id);
                    move_next = true;
                    self.set_bot_role(dte.bot_id, BotRole::Dodging(true));
                },
                Event::NoActionEvent(noe) => {},//println!("Bot ID: {} did nothing!", noe.bot_id),
                Event::MoveEvent(me) => {}, //println!("Bot ID {} moved to x:{}, y:{}", me.bot_id,
                    //me.pos.x, me.pos.y),
                Event::SeeAsteroidEvent(sae) => {
                    match self.current_state.asteroid_map.contains(&MapTile { pos: sae.pos, asteroid: true }) {
                        true => {},
                        false => {
                            self.current_state.asteroid_map.push(MapTile { pos: sae.pos, asteroid: true });
                            // println!("Asteroid stored at x:{}, y:{}. Asteroids saved {}/{:?}",
                            //     sae.pos.x,
                            //     sae.pos.y,
                            //     self.current_state.asteroid_map.len(),
                            //     self.config.asteroids
                            // );
                        }
                    }
                }
            }
        }

        let shoot_deltas = vec![
            Position { x:  1, y:  0 },
            Position { x:  1, y: -1 },
            Position { x: -1, y:  0 },
            Position { x: -1, y:  1 },
            Position { x:  0, y:  1 },
            Position { x:  0, y: -1 },
        ];

        let botpositions: Vec<Position> = self.you.bots.iter().map(|bot| bot.pos).collect();

        let living_bots = self.you.bots.iter().filter(|bot| bot.alive);

        let tile_count = Position { x:0, y:0 }.positions_within(self.config.field_radius as u32).len();

        // Save the asteroid state for each tile
        for bot in self.you.bots.iter().filter(|bot| bot.alive) {
            //println!("Bot visibility {:?}", bot.pos.positions_within(self.config.see as u32));
            for hex in bot.pos.positions_within(self.config.see as u32) {
                match self.current_state.asteroid_map.contains(&MapTile { pos: hex, asteroid: true }) {
                    true => {},
                    false => {
                        // Asteroids added already above, if a tile is missing, add asteroid is false
                        self.current_state.asteroid_map.push(MapTile { pos: hex, asteroid: false });
                        // println!("MapTile INDUBITABLY stored at x:{}, y:{}",
                        //      hex.x,
                        //      hex.y);
                    }
                }
            }
        }

        // println!("{}% of map tiles stored", (self.current_state.asteroid_map.len() as f32 / tile_count as f32) * 100 as f32);

        match (move_next, acquired_target) {
            (false, Some(_)) => shoot_count += 1,
            (_, None) => shoot_count = 0,
            _ => {}
        }

        let actions = living_bots.zip(shoot_deltas.iter().cycle()).map(|(bot, delta)| {
            let other_bots: Vec<Bot> = self.you.bots.clone().into_iter().filter(|a_bot| a_bot.bot_id != bot.bot_id).collect();
            match (move_next, acquired_target) {
                (true, _) => {
                    //println!("bot {:?} has to move from {:?}", bot.bot_id, bot.pos);
                    //println!("allowed move radius {:?}", self.config.move_);
                    let allowed_positions = bot.pos.positions_at(self.config.move_, self.config.field_radius);
                    let asteroids: Vec<MapTile> = self.current_state.asteroid_map.clone().into_iter().filter(|tile| tile.asteroid).collect();

                    let allowed_free_positions = filter_asteroids(allowed_positions.clone(), asteroids.clone());

                    //println!("allowed positions {:?}", allowed_positions);
                    let positions_without_others: Vec<Position> = allowed_free_positions.clone().into_iter().filter(|pos| {
                                other_bots.iter().fold(true, |memo, other| {
                                pos.distance(other.pos) > self.config.see
                        })
                    }).collect();
                    let mut final_positions = allowed_positions.clone();
                    if positions_without_others.len() > 0 {
                            final_positions = positions_without_others.clone();
                    }
                    let chosen = thread_rng().choose(&final_positions).unwrap();
                    //println!("moving to {:?}", chosen);
                    Action::MoveAction(MoveAction {
                                        bot_id: bot.bot_id,
                                        pos: Position { x: chosen.x, y: chosen. y}
                    })
                },
                (false, Some(ref tgtpos)) => {
                    match spotter_bot_id {
                        Some(sbot_id) if sbot_id == bot.bot_id => {
                            let topos = bot.pos.move_away_from(tgtpos, self.config.move_);
                            //println!("Moving {:?} to {:?}", bot.bot_id, topos);
                            Action::MoveAction(MoveAction {
                                bot_id: bot.bot_id,
                                pos: topos
                            })
                        },
                        _ => {
                            //println!("Cannonning");
                            // TODO: make sure spread doesn't hit our spotter
                            let mut cannonpos = Position {
                                x: tgtpos.x + (delta.x * shoot_count),
                                y: tgtpos.y + (delta.y * shoot_count)
                            };
                            println!("cannonpos for {:?} with shotcount {:?} is {:?}", bot.bot_id, shoot_count, cannonpos);
                            let mut bailout_move = false;
                            while cannonpos.contains_any_within(botpositions.clone(), self.config.cannon) {
                                //println!("Moving cannonpos {:?} to avoid hit", cannonpos);
                                if cannonpos.x == tgtpos.x && cannonpos.y == tgtpos.y {
                                    bailout_move = true;
                                    break;
                                }
                                cannonpos = cannonpos.move_towards(*tgtpos, 1);
                                //println!("Moved to {:?}", cannonpos);
                            }
                            match bailout_move {
                                true => {
                                    let allowed_positions = bot.pos.positions_at(self.config.move_, self.config.field_radius);
                                    let asteroids: Vec<MapTile> = self.current_state.asteroid_map.clone().into_iter().filter(|tile| tile.asteroid).collect();

                                    let allowed_free_positions = filter_asteroids(allowed_positions.clone(), asteroids.clone());

                                    // TODO: don't move closer to a friend
                                    // TODO: don't move to an asteroid position
                                    let positions_without_others: Vec<Position> = allowed_free_positions.clone().into_iter().filter(|pos| {
                                        other_bots.iter().fold(true, |memo, other| {
                                            pos.distance(other.pos) > self.config.see
                                        })
                                    }).collect();
                                    let mut final_positions = allowed_positions.clone();
                                    if positions_without_others.len() > 0 {
                                        final_positions = positions_without_others.clone();
                                    }
                                    let chosen = thread_rng().choose(&final_positions).unwrap();
                                    // println!("Positions in total {}, positions far enough of other bots {}",
                                    //          allowed_positions.len(),
                                    //          positions_without_others.len());
                                    //println!("Bot {:?} moving to {:?}", bot.bot_id, chosen);
                                    Action::MoveAction(MoveAction {
                                                        bot_id: bot.bot_id,
                                                        pos: Position { x: chosen.x, y: chosen. y}
                                    })
                                },
                                false => Action::CannonAction(CannonAction {
                                    bot_id: bot.bot_id,
                                    pos: cannonpos
                                })
                            }
                        }
                    }
                },
                (false, None) => {
                    let allowed_positions = bot.pos.positions_at(self.config.move_, self.config.field_radius);
                    let asteroids: Vec<MapTile> = self.current_state.asteroid_map.clone().into_iter().filter(|tile| tile.asteroid).collect();

                    let allowed_free_positions = filter_asteroids(allowed_positions.clone(), asteroids.clone());
                    // TODO: don't move closer to a friend
                    // TODO: don't move to an asteroid position
                    let positions_without_others: Vec<Position> = allowed_free_positions.clone().into_iter().filter(|pos| {
                                other_bots.iter().fold(true, |memo, other| {
                                pos.distance(other.pos) > self.config.see
                        })
                    }).collect();
                    // println!("Positions in total {}, positions far enough of other bots {}",
                    //          allowed_positions.len(),
                    //          positions_without_others.len());

                    let mut final_positions = allowed_positions.clone();
                    if positions_without_others.len() > 0 {
                            final_positions = positions_without_others.clone();
                    }
                    let chosen = thread_rng().choose(&final_positions).unwrap();
                    //println!("Bot {:?} moving to {:?}", bot.bot_id, chosen);
                    Action::MoveAction(MoveAction {
                                        bot_id: bot.bot_id,
                                        pos: Position { x: chosen.x, y: chosen.y}
                    })
                }
            }

        }).collect();

        self.current_state.shoot_count = shoot_count;
        actions
    }

    fn set_state(&mut self, config: GameConfig, you: Team, other_teams: Vec<TeamNoPosNoHp>) {
        self.config = config;
        self.you = you;
        self.other_teams = other_teams;

        self.current_state.bot_states = Vec::new();
        for bot in self.you.bots.iter(){
            self.current_state.bot_states.push(BotState{bot_id:bot.bot_id, current_role: BotRole::SearchMoving(true)});
        }
    }

    #[allow(unused_assignments)]
    fn get_bot_by_id(&mut self, bot_id:u32) -> Option<&Bot>{
        let mut bot_vec: Vec<&Bot> = self.you.bots.iter().filter(|bot| bot.bot_id == bot_id).collect();
        let return_bot: Option<&Bot> = bot_vec.pop();
        return return_bot;
    }

    fn is_on_playing_field(&self, pos: &Position) -> bool {
        pos.distance(Position { x: 0, y: 0 }) <= self.config.field_radius
    }

    #[allow(unused_assignments)]
    fn set_bot_role(&mut self, bot_id: u32, new_role: BotRole){
        let mut found_bot_id = None;
        {
            found_bot_id = match self.get_bot_by_id(bot_id){
                Some(state) => Some(state.bot_id),
                None => None
            }
        }
        match found_bot_id{
            Some(bid) => {
                let index = self.current_state.bot_states.iter().position(|s| s.bot_id == bid);
                self.current_state.bot_states[index.unwrap()].current_role = new_role;
            },
            None => {}//panic, maybe?
        }
    }

    fn get_bot_role(&mut self, bot_id: u32) -> Option<&BotRole>{ //Returns None if it can't find the bot.
        let index = self.current_state.bot_states.iter().position(|s| s.bot_id == bot_id);
        match index{
            Some(i) => Some(&self.current_state.bot_states[i].current_role),
            None => None
        }
    }
}

pub fn from_name(name: String) -> Box<Ai> {
    match name.as_ref() {
        "random" => Box::new(RandomAi { ..Default::default() }),
        _ => panic!("Can't find an AI with name: {}", name)
    }
}

#[test]
fn test_from_name() {
    from_name("random".to_string());
}

#[test]
#[should_panic]
fn test_from_name_nonsense() {
    from_name("not an actual ai".to_string());
}

pub struct MoveAction {
    pub bot_id: u32,
    pub pos: Position
}

pub struct RadarAction {
    pub bot_id: u32,
    pub pos: Position
}

pub struct CannonAction {
    pub bot_id: u32,
    pub pos: Position
}

pub enum Action {
    CannonAction(CannonAction),
    MoveAction(MoveAction),
    RadarAction(RadarAction)
}
