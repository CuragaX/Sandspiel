extern crate cfg_if;
extern crate js_sys;
extern crate wasm_bindgen;
extern crate web_sys;

mod species;
mod utils;

use species::Species;
use std::collections::VecDeque;
use wasm_bindgen::prelude::*;
// use web_sys::console;

#[wasm_bindgen]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wind {
    dx: u8,
    dy: u8,
    pressure: u8,
    density: u8,
}

#[wasm_bindgen]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    species: Species,
    ra: u8,
    rb: u8,
    clock: u8,
}

impl Cell {
    pub fn update(&self, api: SandApi) {
        self.species.update(*self, api);
    }
}

static EMPTY_CELL: Cell = Cell {
    species: Species::Empty,
    ra: 0,
    rb: 0,
    clock: 0,
};

#[wasm_bindgen]
pub struct Universe {
    width: i32,
    height: i32,
    cells: Vec<Cell>,
    undo_stack: VecDeque<Vec<Cell>>,
    winds: Vec<Wind>,
    burns: Vec<Wind>,
    generation: u8,
}

pub struct SandApi<'a> {
    x: i32,
    y: i32,
    universe: &'a mut Universe,
}

impl<'a> SandApi<'a> {
    pub fn get(&mut self, dx: i32, dy: i32) -> Cell {
        if dx > 1 || dx < -1 || dy > 2 || dy < -2 {
            panic!("oob set");
        }
        let nx = self.x + dx;
        let ny = self.y + dy;
        if nx < 0 || nx > self.universe.width - 1 || ny < 0 || ny > self.universe.height - 1 {
            return Cell {
                species: Species::Wall,
                ra: 0,
                rb: 0,
                clock: self.universe.generation,
            };
        }
        self.universe.get_cell(nx, ny)
    }
    pub fn set(&mut self, dx: i32, dy: i32, v: Cell) {
        if dx > 1 || dx < -1 || dy > 2 || dy < -2 {
            panic!("oob set");
        }
        let nx = self.x + dx;
        let ny = self.y + dy;

        if nx < 0 || nx > self.universe.width - 1 || ny < 0 || ny > self.universe.height - 1 {
            return;
        }
        let i = self.universe.get_index(nx, ny);
        // v.clock += 1;
        self.universe.cells[i] = v;
        self.universe.cells[i].clock = self.universe.generation.wrapping_add(1);
    }
    pub fn get_fluid(&mut self) -> Wind {
        let idx = self
            .universe
            .get_index(self.x, self.universe.height - (1 + self.y));

        self.universe.winds[idx]
    }
    pub fn set_fluid(&mut self, v: Wind) {
        let idx = self
            .universe
            .get_index(self.x, self.universe.height - (1 + self.y));

        self.universe.burns[idx] = v;
    }
}

#[wasm_bindgen]
impl Universe {
    pub fn reset(&mut self) {
        for x in 0..self.width {
            for y in 0..self.height {
                let idx = self.get_index(x, y);
                self.cells[idx] = EMPTY_CELL;
            }
        }
    }
    pub fn tick(&mut self) {
        // let mut next = self.cells.clone();
        // let dx = self.winds[(self.width * self.height / 2) as usize].dx;
        // let js: JsValue = (dx).into();
        // console::log_2(&"dx: ".into(), &js);

        for x in 0..self.width {
            for y in 0..self.height {
                let cell = self.get_cell(x, y);
                let wind = self.get_wind(x, y);
                Universe::blow_wind(
                    cell,
                    wind,
                    SandApi {
                        universe: self,
                        x,
                        y,
                    },
                )
            }
        }
        self.generation = self.generation.wrapping_add(1);
        for x in 0..self.width {
            let scanx = if self.generation % 2 == 0 {
                self.width - (1 + x)
            } else {
                x
            };

            for y in 0..self.height {
                let idx = self.get_index(scanx, self.height - (1 + y));
                let cell = self.get_cell(scanx, y);

                self.burns[idx] = Wind {
                    dx: 0,
                    dy: 0,
                    pressure: 0,
                    density: 0,
                };
                Universe::update_cell(
                    cell,
                    SandApi {
                        universe: self,
                        x: scanx,
                        y,
                    },
                );
            }
        }

        self.generation = self.generation.wrapping_add(1);
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn cells(&self) -> *const Cell {
        self.cells.as_ptr()
    }

    pub fn winds(&self) -> *const Wind {
        self.winds.as_ptr()
    }

    pub fn burns(&self) -> *const Wind {
        self.burns.as_ptr()
    }

    pub fn paint(&mut self, x: i32, y: i32, size: i32, species: Species) {
        let radius = size / 2;
        for dx in -radius..radius + 1 {
            for dy in -radius..radius + 1 {
                if dx * dx + dy * dy > (radius * radius) - 1 {
                    continue;
                };
                let px = x + dx;
                let py = y + dy;

                let i = self.get_index(px, py);

                if px < 0 || px > self.width - 1 || py < 0 || py > self.height - 1 {
                    continue;
                }
                if self.get_cell(px, py).species == Species::Empty || species == Species::Empty {
                    self.cells[i] = Cell {
                        species: species,
                        ra: 80
                            + (js_sys::Math::random() * 30.) as u8
                            + ((self.generation % 127) as i8 - 60).abs() as u8,
                        rb: 0,
                        clock: self.generation,
                    }
                }
            }
        }
    }

    pub fn push_undo(&mut self) {
        self.undo_stack.push_front(self.cells.clone());
        self.undo_stack.truncate(50);
    }

    pub fn pop_undo(&mut self) {
        let old_state = self.undo_stack.pop_front();
        match old_state {
            Some(state) => self.cells = state,
            None => (),
        };
    }

    pub fn flush_undos(&mut self) {
        self.undo_stack.clear();
    }

    pub fn new(width: i32, height: i32) -> Universe {
        let cells = (0..width * height)
            .map(|i| {
                if js_sys::Math::random() > 0.9995 && i < width * height / 4 {
                    Cell {
                        species: Species::Seed,
                        ra: 80 + (js_sys::Math::random() * 70.) as u8,
                        rb: 0,
                        clock: 0,
                    }
                } else if js_sys::Math::random() < 0.9 || i < width * height / 3 {
                    EMPTY_CELL
                } else {
                    Cell {
                        species: Species::Sand,
                        ra: 80 + (js_sys::Math::random() * 70.) as u8,
                        rb: 0,
                        clock: 0,
                    }
                }
            })
            .collect();
        let winds: Vec<Wind> = (0..width * height)
            .map(|_i| Wind {
                dx: 0,
                dy: 0,
                pressure: 0,
                density: 0,
            })
            .collect();

        let burns: Vec<Wind> = (0..width * height)
            .map(|_i| Wind {
                dx: 0,
                dy: 0,
                pressure: 0,
                density: 0,
            })
            .collect();

        Universe {
            width,
            height,
            cells,
            undo_stack: VecDeque::with_capacity(50),
            burns,
            winds,
            generation: 0,
        }
    }
}

//private methods
impl Universe {
    fn get_index(&self, x: i32, y: i32) -> usize {
        (x + (y * self.width)) as usize
    }

    fn get_cell(&self, x: i32, y: i32) -> Cell {
        let i = self.get_index(x, y);
        return self.cells[i];
    }

    fn get_wind(&self, x: i32, y: i32) -> Wind {
        let i = self.get_index(x, (self.height - y) - 1);
        return self.winds[i];
    }

    fn blow_wind(cell: Cell, wind: Wind, mut api: SandApi) {
        if cell.clock - api.universe.generation == 1 {
            return;
        }
        if cell.species == Species::Empty {
            return;
        }
        let mut dx = 0;
        let mut dy = 0;

        let threshold = match cell.species {
            Species::Empty => 500,
            Species::Wall => 500,
            Species::Cloner => 500,

            Species::Stone => 70,
            Species::Wood => 70,

            Species::Plant => 60,
            Species::Lava => 60,
            Species::Ice => 60,

            Species::Fungus => 54,

            Species::Oil => 50,

            // Intentionally left out and covered by the default case
            // Species::Water => 40,
            // Species::Acid => 40,

            Species::Seed => 35,

            Species::Sand => 30,
            Species::Mite => 30,
            Species::Firework => 30,

            Species::Dust => 10,
            Species::Fire => 5,
            Species::Gas => 5,
            /*
             Some hacked species values exist outside of the enum values.
             Making sure the default case is emitted allows "BELP" to have a defined wind threshold.
             Originally, threshold was a hardcoded value, so this preserves that original glitch behavior.
             See: https://sandspiel.club/#eMlYGC52XIto0NM1WjaJ
            */
            _ => 40,
        };

        let wx = (wind.dx as i32) - 126;
        let wy = (wind.dy as i32) - 126;

        if wx > threshold {
            dx = 1;
        }
        if wy > threshold {
            dy = -1;
        }
        if wx < -threshold {
            dx = -1;
        }
        if wy < -threshold {
            dy = 1;
        }
        if (dx != 0 || dy != 0) && api.get(dx, dy).species == Species::Empty {
            api.set(0, 0, EMPTY_CELL);
            if dy == -1
                && api.get(dx, -2).species == Species::Empty
                && (cell.species == Species::Sand
                    || cell.species == Species::Water
                    || cell.species == Species::Lava
                    || cell.species == Species::Acid
                    || cell.species == Species::Mite
                    || cell.species == Species::Dust
                    || cell.species == Species::Oil
                    || cell.species == Species::Firework)
            {
                dy = -2;
            }
            api.set(dx, dy, cell);
            return;
        }
    }
    fn update_cell(cell: Cell, api: SandApi) {
        if cell.clock - api.universe.generation == 1 {
            return;
        }

        cell.update(api);
    }
}
