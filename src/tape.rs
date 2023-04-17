use pyo3::prelude::*;
use pyo3::types::PyTuple;

use crate::instrs::{Color, Shift};
use crate::rules::{ApplyRule, Count, Index, Rule};

pub enum ColorCount {
    Just(Color),
    Mult(Color),
}

impl ToPyObject for ColorCount {
    fn to_object(&self, py: Python) -> PyObject {
        match self {
            Self::Just(color) => PyTuple::new(py, vec![color]).to_object(py),
            Self::Mult(color) => color.into_py(py),
        }
    }
}

pub struct Signature {
    scan: Color,
    lspan: Vec<ColorCount>,
    rspan: Vec<ColorCount>,
}

impl IntoPy<PyObject> for Signature {
    fn into_py(self, py: Python) -> PyObject {
        (
            self.scan,
            PyTuple::new(py, self.lspan),
            PyTuple::new(py, self.rspan),
        )
            .into_py(py)
    }
}

/*****************************************************************/

type Tag = u8;

struct TagBlock {
    color: Color,
    count: Count,
    tags: Vec<Tag>,
}

type TagBlockSrlzd = (Color, Count, Vec<Tag>);

impl From<&TagBlock> for TagBlockSrlzd {
    fn from(block: &TagBlock) -> Self {
        (block.color, block.count.clone(), block.tags.clone())
    }
}

#[pyclass]
pub struct TagTape {
    lspan: Vec<TagBlock>,

    #[pyo3(get, set)]
    pub scan: Color,

    rspan: Vec<TagBlock>,

    #[pyo3(get, set)]
    scan_info: Vec<Tag>,
}

#[pymethods]
impl TagTape {
    #[new]
    fn new(
        lspan: Vec<(Color, Count, Vec<Tag>)>,
        scan: Color,
        rspan: Vec<(Color, Count, Vec<Tag>)>,
    ) -> Self {
        Self {
            lspan: lspan
                .into_iter()
                .map(|(color, count, tags)| TagBlock { color, count, tags })
                .collect(),
            scan,
            rspan: rspan
                .into_iter()
                .map(|(color, count, tags)| TagBlock { color, count, tags })
                .collect(),
            scan_info: vec![],
        }
    }

    #[getter]
    fn lspan(&self) -> Vec<(Color, Count, Vec<Tag>)> {
        self.lspan.iter().map(std::convert::Into::into).collect()
    }

    #[getter]
    fn rspan(&self) -> Vec<(Color, Count, Vec<Tag>)> {
        self.rspan.iter().map(std::convert::Into::into).collect()
    }

    #[getter]
    pub fn missing_tags(&self) -> bool {
        [&self.lspan, &self.rspan].iter().any(|span| {
            span.iter()
                .any(|block| block.count > Count::from(1) && block.tags.len() != 1)
        })
    }

    #[getter]
    pub fn blank(&self) -> bool {
        self.scan == 0 && self.lspan.is_empty() && self.rspan.is_empty()
    }

    pub fn at_edge(&self, edge: Shift) -> bool {
        self.scan == 0 && (if edge { &self.rspan } else { &self.lspan }).is_empty()
    }

    #[getter]
    pub fn counts(&self) -> (Vec<Count>, Vec<Count>) {
        (
            self.lspan.iter().map(|b| b.count.clone()).collect(),
            self.rspan.iter().map(|b| b.count.clone()).collect(),
        )
    }

    #[getter]
    pub fn signature(&self) -> Signature {
        Signature {
            scan: self.scan,
            lspan: self
                .lspan
                .iter()
                .map(|block| {
                    let cons = if block.count == Count::from(1) {
                        ColorCount::Just
                    } else {
                        ColorCount::Mult
                    };
                    cons(block.color)
                })
                .collect(),
            rspan: self
                .rspan
                .iter()
                .map(|block| {
                    let cons = if block.count == Count::from(1) {
                        ColorCount::Just
                    } else {
                        ColorCount::Mult
                    };
                    cons(block.color)
                })
                .collect(),
        }
    }

    pub fn step(&mut self, shift: bool, color: Color, mut skip: bool) {
        let (pull, push) = if shift {
            (&mut self.rspan, &mut self.lspan)
        } else {
            (&mut self.lspan, &mut self.rspan)
        };

        skip = skip && !pull.is_empty() && pull[0].color == self.scan;

        let mut push_block = if skip { Some(pull.remove(0)) } else { None };

        let stepped = if push_block.is_none() {
            Count::from(1)
        } else {
            Count::from(1) + push_block.as_mut().unwrap().count.clone()
        };

        let mut scan_info: Vec<Tag> = vec![];

        let next_scan: Color;

        let mut dec_pull: bool = false;
        let mut inc_push: bool = false;

        if pull.is_empty() {
            next_scan = 0;
        } else {
            let next_pull = &mut pull[0];
            next_scan = next_pull.color;

            if next_pull.count > Count::from(1) {
                next_pull.count -= 1;
                dec_pull = true;
            } else {
                let popped = pull.remove(0);

                let popped_has_tags = !popped.tags.is_empty();

                if popped_has_tags {
                    scan_info.extend(popped.tags.clone());
                }

                if push_block.is_none() {
                    push_block = Some(popped);
                    push_block.as_mut().unwrap().count = Count::from(0);
                }

                if popped_has_tags {
                    push_block.as_mut().unwrap().tags.clear();
                }
            }
        }

        if !push.is_empty() && push[0].color == color {
            inc_push = true;
            let top_block = &mut push[0];
            top_block.count += stepped;
            top_block.tags.extend(self.scan_info.clone());

            if let Some(push_block) = &push_block {
                top_block.tags.extend(push_block.tags.clone());
            }
        } else {
            if push_block.is_none() {
                let mut tags = vec![];
                if !push.is_empty() && color != self.scan {
                    let top_tags = &mut push[0].tags;
                    if top_tags.len() > 1 {
                        tags.push(top_tags.pop().unwrap());
                    }
                }

                if dec_pull {
                    tags.extend(self.scan_info.clone());
                }

                push_block = Some(TagBlock {
                    color,
                    count: Count::from(1),
                    tags,
                });
            } else {
                let block = push_block.as_mut().unwrap();
                block.color = color;
                block.count += 1;

                if !push.is_empty() {
                    let top_tags = &mut push[0].tags;
                    if top_tags.len() > 1 {
                        block.tags.push(top_tags.pop().unwrap());
                    }
                }

                if !self.scan_info.is_empty() {
                    block.tags.extend(self.scan_info.clone());
                }
            }

            if !push.is_empty()
                || color != 0
                || !push_block.as_ref().unwrap().tags.is_empty()
                || skip
            {
                if color == 0 && push.is_empty() {
                    push_block.as_mut().unwrap().count = Count::from(1);
                }

                push.insert(0, push_block.take().unwrap());

                if !self.scan_info.is_empty() && push[0].tags.is_empty() {
                    push[0].tags.extend(self.scan_info.clone());
                }
            }
        }

        if inc_push && push[0].tags.is_empty() {
            push[0].tags.extend(scan_info.clone());
        } else {
            self.scan_info = scan_info;
        }

        self.scan = next_scan;
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn apply_rule(&mut self, rule: Rule) -> PyResult<Option<Count>> {
        self.apply_rule_rs(&rule)
    }
}

impl ApplyRule for TagTape {
    fn __getitem__(&self, index: &Index) -> Count {
        let (side, pos) = index;
        let span = if *side { &self.rspan } else { &self.lspan };
        span[*pos].count.clone()
    }

    fn __setitem__(&mut self, index: &Index, val: Count) {
        let (side, pos) = index;
        let span = if *side {
            &mut self.rspan
        } else {
            &mut self.lspan
        };
        span[*pos].count = val;
    }
}

/*****************************************************************/

struct EnumBlock {
    color: Color,
    count: Count,
    enums: Option<(Shift, usize)>,
}

#[pyclass]
pub struct EnumTape {
    lspan: Vec<EnumBlock>,

    #[pyo3(get, set)]
    pub scan: Color,

    rspan: Vec<EnumBlock>,

    offsets: [usize; 2],

    edges: [bool; 2],
}

#[pymethods]
impl EnumTape {
    #[new]
    fn new(lspan: Vec<(Color, Count)>, scan: Color, rspan: Vec<(Color, Count)>) -> Self {
        Self {
            lspan: lspan
                .into_iter()
                .enumerate()
                .map(|(i, (color, count))| EnumBlock {
                    color,
                    count,
                    enums: Some((false, i + 1)),
                })
                .collect(),
            scan,
            rspan: rspan
                .into_iter()
                .enumerate()
                .map(|(i, (color, count))| EnumBlock {
                    color,
                    count,
                    enums: Some((true, (i + 1))),
                })
                .collect(),
            offsets: [0, 0],
            edges: [false, false],
        }
    }

    #[getter]
    pub const fn offsets(&self) -> (usize, usize) {
        (self.offsets[0], self.offsets[1])
    }

    #[getter]
    pub const fn edges(&self) -> (bool, bool) {
        (self.edges[0], self.edges[1])
    }

    #[getter]
    pub fn lspan(&self) -> Vec<(Color, Count)> {
        self.lspan
            .iter()
            .map(|block| (block.color, block.count.clone()))
            .collect()
    }

    #[getter]
    pub fn rspan(&self) -> Vec<(Color, Count)> {
        self.rspan
            .iter()
            .map(|block| (block.color, block.count.clone()))
            .collect()
    }

    #[getter]
    pub fn signature(&self) -> Signature {
        Signature {
            scan: self.scan,
            lspan: self
                .lspan
                .iter()
                .map(|block| {
                    let cons = if block.count == Count::from(1) {
                        ColorCount::Just
                    } else {
                        ColorCount::Mult
                    };
                    cons(block.color)
                })
                .collect(),
            rspan: self
                .rspan
                .iter()
                .map(|block| {
                    let cons = if block.count == Count::from(1) {
                        ColorCount::Just
                    } else {
                        ColorCount::Mult
                    };
                    cons(block.color)
                })
                .collect(),
        }
    }

    pub fn step(&mut self, shift: bool, color: Color, skip: bool) {
        let (pull, push) = if shift {
            (&mut self.rspan, &mut self.lspan)
        } else {
            (&mut self.lspan, &mut self.rspan)
        };

        if pull.is_empty() {
            self.edges[usize::from(shift)] = true;
        } else {
            if let Some(enums) = pull[0].enums {
                let (ind, offset) = enums;

                if offset > self.offsets[usize::from(ind)] {
                    self.offsets[usize::from(ind)] = offset;
                }
            }

            if skip && pull[0].color == self.scan {
                if pull.len() <= 1 {
                    self.edges[usize::from(shift)] = true;
                } else if let Some(next_block) = pull[1].enums {
                    let (ind, offset) = next_block;

                    if offset > self.offsets[usize::from(ind)] {
                        self.offsets[usize::from(ind)] = offset;
                    }
                }
            }
        }

        if !push.is_empty() {
            if let Some(enums) = push[0].enums {
                let (ind, offset) = enums;

                if offset > self.offsets[usize::from(ind)] {
                    self.offsets[usize::from(ind)] = offset;
                }
            }
        }

        let mut push_block = if skip && !pull.is_empty() && pull[0].color == self.scan {
            Some(pull.remove(0))
        } else {
            None
        };

        let stepped = push_block.as_ref().map_or_else(
            || Count::from(1),
            |block| Count::from(1) + block.count.clone(),
        );

        let next_scan: Color;

        if pull.is_empty() {
            next_scan = 0;
        } else {
            let next_pull = &mut pull[0];

            next_scan = next_pull.color;

            if next_pull.count > Count::from(1) {
                next_pull.count -= 1;
            } else {
                let mut popped = pull.remove(0);

                if push_block.is_none() {
                    popped.count = Count::from(0);
                    push_block = Some(popped);
                }
            }
        }

        if !push.is_empty() && push[0].color == color {
            push[0].count += stepped;
        } else {
            if let Some(block) = &mut push_block {
                block.color = color;
                block.count += 1;
            } else {
                push_block = Some(EnumBlock {
                    color,
                    count: Count::from(1),
                    enums: None,
                });
            }

            if !push.is_empty() || color != 0 {
                if let Some(block) = push_block {
                    push.insert(0, block);
                }
            }
        }

        self.scan = next_scan;
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn apply_rule(&mut self, rule: Rule) -> PyResult<Option<Count>> {
        self.apply_rule_rs(&rule)
    }
}

impl ApplyRule for EnumTape {
    fn __getitem__(&self, index: &Index) -> Count {
        let (side, pos) = index;
        let span = if *side { &self.rspan } else { &self.lspan };
        span[*pos].count.clone()
    }

    fn __setitem__(&mut self, index: &Index, val: Count) {
        let (side, pos) = index;
        let span = if *side {
            &mut self.rspan
        } else {
            &mut self.lspan
        };
        span[*pos].count = val;
    }
}
