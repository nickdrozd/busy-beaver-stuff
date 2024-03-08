use crate::instrs::{Color, Shift};

pub type Count = u64;

/**************************************/

type TupleBlock = (Color, Count);
pub type TupleTape = (Vec<TupleBlock>, Color, Vec<TupleBlock>);

pub struct Block {
    pub color: Color,
    pub count: Count,
}

impl From<TupleBlock> for Block {
    fn from((color, count): TupleBlock) -> Self {
        Self::new(color, count)
    }
}

impl Block {
    pub const fn new(color: Color, count: Count) -> Self {
        Self { color, count }
    }
}

/**************************************/

pub struct Tape {
    lspan: Vec<Block>,

    pub scan: Color,

    rspan: Vec<Block>,
}

impl Tape {
    pub fn init_stepped() -> Self {
        Self {
            lspan: vec![Block::new(1, 1)],
            scan: 0,
            rspan: vec![],
        }
    }

    pub fn from_tuples(tape: TupleTape) -> Self {
        let (lspan, scan, rspan) = tape;

        Self {
            lspan: lspan.into_iter().map(std::convert::Into::into).collect(),
            scan,
            rspan: rspan.into_iter().map(std::convert::Into::into).collect(),
        }
    }

    pub fn at_edge(&self, edge: Shift) -> bool {
        self.scan == 0 && (if edge { &self.rspan } else { &self.lspan }).is_empty()
    }

    pub fn blank(&self) -> bool {
        self.scan == 0 && self.lspan.is_empty() && self.rspan.is_empty()
    }

    pub fn backstep(&mut self, shift: Shift, color: Color) {
        let _ = self.step(!shift, self.scan, false);

        self.scan = color;
    }

    pub fn step(&mut self, shift: Shift, color: Color, skip: bool) -> Count {
        let (pull, push) = if shift {
            (&mut self.rspan, &mut self.lspan)
        } else {
            (&mut self.lspan, &mut self.rspan)
        };

        let mut push_block = if skip && !pull.is_empty() && pull[0].color == self.scan {
            Some(pull.remove(0))
        } else {
            None
        };

        let stepped = push_block
            .as_ref()
            .map_or_else(|| 1, |block| 1 + block.count);

        let next_scan: Color;

        if pull.is_empty() {
            next_scan = 0;
        } else {
            let next_pull = &mut pull[0];

            next_scan = next_pull.color;

            if next_pull.count > 1 {
                next_pull.count -= 1;
            } else {
                let mut popped = pull.remove(0);

                if push_block.is_none() {
                    popped.count = 0;
                    push_block = Some(popped);
                }
            }
        }

        if !push.is_empty() && push[0].color == color {
            push[0].count += stepped;
        } else if !push.is_empty() || color != 0 {
            if let Some(block) = &mut push_block {
                block.color = color;
                block.count += 1;
            } else {
                push_block = Some(Block::new(color, 1));
            }

            if let Some(block) = push_block {
                push.insert(0, block);
            }
        }

        self.scan = next_scan;

        stepped
    }
}
