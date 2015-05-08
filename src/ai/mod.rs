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
}

#[derive(Debug, Default)]
struct State {
    //global state
    last_target: Option<Position>,
    //bot state
    bot_states: Vec<BotState>
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

impl Ai for RandomAi {

    #[allow(unused_variables, unused_assignments)]
    fn respond(&mut self, events: Vec<Event>) -> Vec<Action>  {

        let mut acquired_target: Option<Position> = None;

        for event in events.into_iter()
        {
            match event {
                Event::DamagedEvent(de) => { //took damage
                    println!("Bot ID: {} took {} damage!",de.bot_id, de.damage);

                    let mut damaged_bot_id = None;
                    {
                        let damaged_bot = self.get_bot_by_id(de.bot_id);
                        match damaged_bot{
                            Some(b) => damaged_bot_id = Some(b.bot_id),
                            None => damaged_bot_id = None
                        }
                    }
                    match damaged_bot_id {
                        Some(bid) =>{
                            let index = self.current_state.bot_states.iter().position(|s| s.bot_id == bid);
                            //self.current_state.bot_states[index.unwrap()].is_running = true ;
                        },
                        None => {}
                    }
                },
                Event::HitEvent(he) => { //hit another ship
                     acquired_target = match self.current_state.last_target{
                         Some(ref tar) => Some(Position {x: tar.x, y:tar.y}),
                         None => None
                     };
                     println!("Bot ID: {} hit enemy bot: {}", he.source, he.bot_id);
                },
                Event::DieEvent(de) => println!("Bot ID: {} died.", de.bot_id),
                Event::SeeEvent(se) =>{
                    acquired_target = Some(Position{x: se.pos.x, y: se.pos.y});
                    match acquired_target{
                        Some(ref tar) => self.current_state.last_target = Some(Position{x: tar.x, y:tar.y}) ,
                        None => {}
                    };
                     println!("Bot ID: {} saw bot: {} at x:{}, y:{}", se.bot_id,
                    se.source, se.pos.x, se.pos.y)
                },
                Event::RadarEchoEvent(ree) => {
                    acquired_target = Some(Position { x: ree.pos.x, y: ree.pos.y });
                    match acquired_target{
                        Some(ref tar) => self.current_state.last_target = Some(Position {x:tar.x, y:tar.y}),
                        None => {}
                    };
                    println!("An enemy was radar-detected in a radius centered at x:{}, y:{}",
                        ree.pos.x, ree.pos.y)
                },
                Event::DetectedEvent(dte) => {
                    println!("Bot ID: {} got radar-detected!", dte.bot_id);
                    self.set_bot_role(dte.bot_id, BotRole::Dodging(true));
                },
                Event::NoActionEvent(noe) => println!("Bot ID: {} did nothing!", noe.bot_id),
                Event::MoveEvent(me) => println!("Bot ID {} moved to x:{}, y:{}", me.bot_id,
                    me.pos.x, me.pos.y),
                Event::SeeAsteroidEvent(sae) => println!("Asteroid seen at x:{}, y:{}", sae.pos.x, sae.pos.y)
            }
        }

        println!("Last target {:?}", self.current_state.last_target);

        let random_x_offset = thread_rng().gen_range(0, 2);

        let shoot_deltas = vec![
            Position { x: -1, y:  1 },
            Position { x:  0, y:  1 },
            Position { x: random_x_offset, y: -1 },
        ];

        let living_bots = self.you.bots.iter().filter(|bot| bot.alive);

        let move_next = true;

        living_bots.zip(shoot_deltas.iter()).map(|(bot, delta)| {
            match move_next {
                true => {
                    println!("bot {:?} has to move from {:?}", bot.bot_id, bot.pos);
                    println!("allowed move radius {:?}", self.config.move_);
                    let allowed_positions = bot.pos.positions_at(self.config.move_);
                    println!("allowed positions {:?}", allowed_positions);
                    let chosen = thread_rng().choose(&allowed_positions).unwrap();
                    println!("moving to {:?}", chosen);
                    Action::MoveAction(MoveAction {
                                        bot_id: bot.bot_id,
                                        pos: Position { x: chosen.x, y: chosen. y}
                    })
                },
                false => match acquired_target {
                    Some(ref pos) => {
                            println!("Cannonning");
                            Action::CannonAction(CannonAction {
                                bot_id: bot.bot_id,
                                pos: Position {
                                    x: pos.x + delta.x,
                                    y: pos.y + delta.y
                                }
                            })
                        },
                    None => {
                            println!("Radarign");
                            Action::RadarAction(RadarAction {
                                bot_id: bot.bot_id,
                                pos: Position {
                                    x: thread_rng().gen_range(-self.config.field_radius, self.config.field_radius),
                                    y: thread_rng().gen_range(-self.config.field_radius, self.config.field_radius)
                                }
                            })
                        }
                }
            }

        }).collect()
    }

    fn set_state(&mut self, config: GameConfig, you: Team, other_teams: Vec<TeamNoPosNoHp>) {
        self.config = config;
        self.you = you;
        self.other_teams = other_teams;

        self.current_state.bot_states = Vec::new();
        for bot in self.you.bots.iter(){
            //self.current_state.bot_states.push(BotState{bot_id:bot.bot_id, is_scanning: false, is_running: false, is_shooting: false});
        }
    }

    #[allow(unused_assignments)]
    fn get_bot_by_id(&mut self, bot_id:u32) -> Option<&Bot>{
        let mut bot_vec: Vec<&Bot> = self.you.bots.iter().filter(|bot| bot.bot_id == bot_id).collect();
        let return_bot: Option<&Bot> = bot_vec.pop();
        return return_bot;
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
