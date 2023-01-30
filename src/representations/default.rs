use byteorder::{LittleEndian, WriteBytesExt};
use bytes::{Buf, BufMut};
use std::{cmp::Ordering, io::Cursor};

use crate::state::{ResettableBuffer, State};

use super::{
    number::{BorrowedNumber, Number, PackedRationalNumberReader, PackedRationalNumberWriter},
    tree::AtomTree,
    Add, Atom, AtomView, Convert, Fun, Identifier, ListIterator, Mul, Num, OwnedAdd, OwnedAtom,
    OwnedFun, OwnedMul, OwnedNum, OwnedPow, OwnedVar, Pow, Var,
};

const NUM_ID: u8 = 1;
const VAR_ID: u8 = 2;
const FUN_ID: u8 = 3;
const MUL_ID: u8 = 4;
const POW_ID: u8 = 5;
const ADD_ID: u8 = 6;
const TYPE_MASK: u8 = 0b00000111;
const DIRTY_FLAG: u8 = 0b10000000;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct DefaultRepresentation {}

#[derive(Debug, Clone)]
pub struct OwnedNumD {
    data: Vec<u8>,
}

impl OwnedNum for OwnedNumD {
    type P = DefaultRepresentation;

    fn from_number(&mut self, num: Number) {
        self.data.clear();
        self.data.put_u8(NUM_ID);
        num.write_packed(&mut self.data);
    }

    fn from_view<'a>(&mut self, a: &NumViewD<'a>) {
        self.data.clear();
        self.data.extend(a.data);
    }

    fn add<'a>(&mut self, other: &NumViewD<'a>, state: &State) {
        let nv = self.to_num_view();
        let a = nv.get_number_view();
        let b = other.get_number_view();
        let n = a.add(&b, state);

        self.data.truncate(1);
        n.write_packed(&mut self.data);
    }

    fn mul<'a>(&mut self, other: &NumViewD<'a>, state: &State) {
        let nv = self.to_num_view();
        let a = nv.get_number_view();
        let b = other.get_number_view();
        let n = a.mul(&b, state);

        self.data.truncate(1);
        n.write_packed(&mut self.data);
    }

    fn to_num_view(&self) -> NumViewD {
        assert!(self.data[0] & TYPE_MASK == NUM_ID);
        NumViewD { data: &self.data }
    }
}

impl Convert<DefaultRepresentation> for OwnedNumD {
    fn to_owned_var(mut self) -> OwnedVarD {
        self.data.clear();
        OwnedVarD { data: self.data }
    }

    fn to_owned_pow(mut self) -> OwnedPowD {
        self.data.clear();
        OwnedPowD { data: self.data }
    }

    fn to_owned_num(mut self) -> OwnedNumD {
        self.data.clear();
        OwnedNumD { data: self.data }
    }

    fn to_owned_fun(mut self) -> OwnedFunD {
        self.data.clear();
        OwnedFunD { data: self.data }
    }

    fn to_owned_add(mut self) -> OwnedAddD {
        self.data.clear();
        OwnedAddD { data: self.data }
    }

    fn to_owned_mul(mut self) -> OwnedMulD {
        self.data.clear();
        OwnedMulD { data: self.data }
    }
}

impl ResettableBuffer for OwnedNumD {
    fn new() -> Self {
        OwnedNumD { data: vec![] }
    }

    fn reset(&mut self) {
        self.data.clear();
    }
}

#[derive(Debug, Clone)]
pub struct OwnedVarD {
    data: Vec<u8>,
}

impl OwnedVar for OwnedVarD {
    type P = DefaultRepresentation;

    fn from_id(&mut self, id: Identifier) {
        self.data.clear();
        self.data.put_u8(VAR_ID);
        (id.to_u32() as u64, 1).write_packed(&mut self.data);
    }

    fn to_var_view<'a>(&'a self) -> <Self::P as Atom>::V<'a> {
        VarViewD { data: &self.data }
    }

    fn from_view<'a>(&mut self, view: &VarViewD) {
        self.data.clear();
        self.data.extend(view.data);
    }
}

impl Convert<DefaultRepresentation> for OwnedVarD {
    fn to_owned_var(mut self) -> OwnedVarD {
        self.data.clear();
        OwnedVarD { data: self.data }
    }

    fn to_owned_pow(mut self) -> OwnedPowD {
        self.data.clear();
        OwnedPowD { data: self.data }
    }

    fn to_owned_num(mut self) -> OwnedNumD {
        self.data.clear();
        OwnedNumD { data: self.data }
    }

    fn to_owned_fun(mut self) -> OwnedFunD {
        self.data.clear();
        OwnedFunD { data: self.data }
    }

    fn to_owned_add(mut self) -> OwnedAddD {
        self.data.clear();
        OwnedAddD { data: self.data }
    }

    fn to_owned_mul(mut self) -> OwnedMulD {
        self.data.clear();
        OwnedMulD { data: self.data }
    }
}

impl ResettableBuffer for OwnedVarD {
    fn new() -> Self {
        OwnedVarD { data: vec![] }
    }

    fn reset(&mut self) {
        self.data.clear();
    }
}

#[derive(Debug, Clone)]
pub struct OwnedFunD {
    data: Vec<u8>,
}

impl OwnedFun for OwnedFunD {
    type P = DefaultRepresentation;

    fn from_name(&mut self, id: Identifier) {
        self.data.clear();
        self.data.put_u8(VAR_ID);
        (id.to_u32() as u64, 1).write_packed(&mut self.data);
    }

    fn set_dirty(&mut self, dirty: bool) {
        if dirty {
            self.data[0] &= DIRTY_FLAG;
        } else {
            self.data[0] &= !DIRTY_FLAG;
        }
    }

    fn add_arg(&mut self, other: AtomView<Self::P>) {
        // TODO: update the size
        self.data.extend(other.get_data());
        todo!()
    }

    fn to_fun_view<'a>(&'a self) -> <Self::P as Atom>::F<'a> {
        FnViewD { data: &self.data }
    }

    fn from_view<'a>(&mut self, view: &<Self::P as Atom>::F<'a>) {
        self.data.clear();
        self.data.extend(view.data);
    }
}

impl Convert<DefaultRepresentation> for OwnedFunD {
    fn to_owned_var(mut self) -> OwnedVarD {
        self.data.clear();
        OwnedVarD { data: self.data }
    }

    fn to_owned_pow(mut self) -> OwnedPowD {
        self.data.clear();
        OwnedPowD { data: self.data }
    }

    fn to_owned_num(mut self) -> OwnedNumD {
        self.data.clear();
        OwnedNumD { data: self.data }
    }

    fn to_owned_fun(mut self) -> OwnedFunD {
        self.data.clear();
        OwnedFunD { data: self.data }
    }

    fn to_owned_add(mut self) -> OwnedAddD {
        self.data.clear();
        OwnedAddD { data: self.data }
    }

    fn to_owned_mul(mut self) -> OwnedMulD {
        self.data.clear();
        OwnedMulD { data: self.data }
    }
}

impl ResettableBuffer for OwnedFunD {
    fn new() -> Self {
        OwnedFunD { data: vec![] }
    }

    fn reset(&mut self) {
        self.data.clear();
    }
}

#[derive(Debug, Clone)]
pub struct OwnedPowD {
    data: Vec<u8>,
}

impl OwnedPow for OwnedPowD {
    type P = DefaultRepresentation;

    fn from_base_and_exp(&mut self, base: AtomView<Self::P>, exp: AtomView<Self::P>) {
        self.data.clear();
        self.data.put_u8(POW_ID);
        self.data.extend(base.get_data());
        self.data.extend(exp.get_data());
    }

    fn set_dirty(&mut self, dirty: bool) {
        if dirty {
            self.data[0] &= DIRTY_FLAG;
        } else {
            self.data[0] &= !DIRTY_FLAG;
        }
    }

    fn to_pow_view<'a>(&'a self) -> <Self::P as Atom>::P<'a> {
        PowViewD { data: &self.data }
    }

    fn from_view<'a>(&mut self, view: &<Self::P as Atom>::P<'a>) {
        self.data.clear();
        self.data.extend(view.data);
    }
}

impl Convert<DefaultRepresentation> for OwnedPowD {
    fn to_owned_var(mut self) -> OwnedVarD {
        self.data.clear();
        OwnedVarD { data: self.data }
    }

    fn to_owned_pow(mut self) -> OwnedPowD {
        self.data.clear();
        OwnedPowD { data: self.data }
    }

    fn to_owned_num(mut self) -> OwnedNumD {
        self.data.clear();
        OwnedNumD { data: self.data }
    }

    fn to_owned_fun(mut self) -> OwnedFunD {
        self.data.clear();
        OwnedFunD { data: self.data }
    }

    fn to_owned_add(mut self) -> OwnedAddD {
        self.data.clear();
        OwnedAddD { data: self.data }
    }

    fn to_owned_mul(mut self) -> OwnedMulD {
        self.data.clear();
        OwnedMulD { data: self.data }
    }
}

impl ResettableBuffer for OwnedPowD {
    fn new() -> Self {
        OwnedPowD { data: vec![] }
    }

    fn reset(&mut self) {
        self.data.clear();
    }
}

pub struct OwnedMulD {
    data: Vec<u8>,
}

impl OwnedMul for OwnedMulD {
    type P = DefaultRepresentation;

    fn extend<'a>(&mut self, other: AtomView<'a, DefaultRepresentation>) {
        if self.data.is_empty() {
            self.data.put_u8(MUL_ID);
            self.data.put_u32_le(0 as u32);
            (0u64, 1).write_packed(&mut self.data);
        }

        // may increase size of the num of args
        let c = &self.data[1 + 4..];

        let buf_pos = 1 + 4;

        let mut n_args;
        (n_args, _, _) = c.get_frac_i64();

        match other {
            AtomView::Mul(_t) => {
                todo!();
            }
            _ => {
                n_args += 1;
                self.data.extend(other.get_data());
            }
        }

        // FIXME: this may overwrite the rest of the term
        // assume for now it does not
        (n_args, 1).write_packed_fixed(&mut self.data[1 + 4..]);

        let new_buf_pos = self.data.len();

        let mut cursor = &mut self.data[1..];
        cursor
            .write_u32::<LittleEndian>((new_buf_pos - buf_pos) as u32)
            .unwrap();
    }

    fn to_mul_view<'a>(&'a self) -> <Self::P as Atom>::M<'a> {
        MulViewD { data: &self.data }
    }

    fn from_view<'a>(&mut self, view: &<Self::P as Atom>::M<'a>) {
        self.data.clear();
        self.data.extend(view.data);
    }
}

impl Convert<DefaultRepresentation> for OwnedMulD {
    fn to_owned_var(mut self) -> OwnedVarD {
        self.data.clear();
        OwnedVarD { data: self.data }
    }

    fn to_owned_pow(mut self) -> OwnedPowD {
        self.data.clear();
        OwnedPowD { data: self.data }
    }

    fn to_owned_num(mut self) -> OwnedNumD {
        self.data.clear();
        OwnedNumD { data: self.data }
    }

    fn to_owned_fun(mut self) -> OwnedFunD {
        self.data.clear();
        OwnedFunD { data: self.data }
    }

    fn to_owned_add(mut self) -> OwnedAddD {
        self.data.clear();
        OwnedAddD { data: self.data }
    }

    fn to_owned_mul(mut self) -> OwnedMulD {
        self.data.clear();
        OwnedMulD { data: self.data }
    }
}

impl ResettableBuffer for OwnedMulD {
    fn new() -> Self {
        OwnedMulD { data: vec![] }
    }

    fn reset(&mut self) {
        self.data.clear();
    }
}

pub struct OwnedAddD {
    data: Vec<u8>,
}

impl OwnedAdd for OwnedAddD {
    type P = DefaultRepresentation;

    fn extend<'a>(&mut self, other: AtomView<'a, DefaultRepresentation>) {
        if self.data.is_empty() {
            self.data.put_u8(ADD_ID);
            self.data.put_u32_le(0 as u32);
            (0u64, 1).write_packed(&mut self.data);
        }

        // may increase size of the num of args
        let c = &self.data[1 + 4..];

        let buf_pos = 1 + 4;

        let mut n_args;
        (n_args, _, _) = c.get_frac_i64();

        match other {
            AtomView::Add(_t) => {
                todo!();
            }
            _ => {
                n_args += 1;
                self.data.extend(other.get_data());
            }
        }

        // FIXME: this may overwrite the rest of the term
        // assume for now it does not
        (n_args, 1).write_packed_fixed(&mut self.data[1 + 4..]);

        let new_buf_pos = self.data.len();

        let mut cursor = &mut self.data[1..];
        cursor
            .write_u32::<LittleEndian>((new_buf_pos - buf_pos) as u32)
            .unwrap();
    }

    fn to_add_view<'a>(&'a self) -> <Self::P as Atom>::A<'a> {
        AddViewD { data: &self.data }
    }

    fn from_view<'a>(&mut self, view: &<Self::P as Atom>::A<'a>) {
        self.data.clear();
        self.data.extend(view.data);
    }
}

impl Convert<DefaultRepresentation> for OwnedAddD {
    fn to_owned_var(mut self) -> OwnedVarD {
        self.data.clear();
        OwnedVarD { data: self.data }
    }

    fn to_owned_pow(mut self) -> OwnedPowD {
        self.data.clear();
        OwnedPowD { data: self.data }
    }

    fn to_owned_num(mut self) -> OwnedNumD {
        self.data.clear();
        OwnedNumD { data: self.data }
    }

    fn to_owned_fun(mut self) -> OwnedFunD {
        self.data.clear();
        OwnedFunD { data: self.data }
    }

    fn to_owned_add(mut self) -> OwnedAddD {
        self.data.clear();
        OwnedAddD { data: self.data }
    }

    fn to_owned_mul(mut self) -> OwnedMulD {
        self.data.clear();
        OwnedMulD { data: self.data }
    }
}

impl ResettableBuffer for OwnedAddD {
    fn new() -> Self {
        let mut data = Vec::new();
        data.put_u8(ADD_ID);
        data.put_u32_le(0 as u32);
        (0u64, 1).write_packed(&mut data);

        OwnedAddD { data }
    }

    fn reset(&mut self) {
        self.data.clear();
        self.data.put_u8(ADD_ID);
        self.data.put_u32_le(0 as u32);
        (0u64, 1).write_packed(&mut self.data);
    }
}

impl Atom for DefaultRepresentation {
    type N<'a> = NumViewD<'a>;
    type V<'a> = VarViewD<'a>;
    type F<'a> = FnViewD<'a>;
    type P<'a> = PowViewD<'a>;
    type M<'a> = MulViewD<'a>;
    type A<'a> = AddViewD<'a>;
    type ON = OwnedNumD;
    type OV = OwnedVarD;
    type OF = OwnedFunD;
    type OP = OwnedPowD;
    type OM = OwnedMulD;
    type OA = OwnedAddD;
}

impl<'a> Var<'a> for VarViewD<'a> {
    type P = DefaultRepresentation;

    #[inline]
    fn get_name(&self) -> Identifier {
        Identifier::from((&self.data[1..]).get_frac_i64().0 as u32)
    }

    fn to_view(&self) -> AtomView<'a, Self::P> {
        AtomView::Var(self.clone())
    }
}

impl OwnedAtom<DefaultRepresentation> {
    pub fn from_tree(&mut self, atom: &AtomTree) {
        match atom {
            AtomTree::Var(_) => {
                let x = self.transform_to_var();
                Self::linearize(&mut x.data, atom);
            }
            AtomTree::Fn(_, _) => {
                let x = self.transform_to_fun();
                Self::linearize(&mut x.data, atom);
            }
            AtomTree::Num(_) => {
                let x = self.transform_to_num();
                Self::linearize(&mut x.data, atom);
            }
            AtomTree::Pow(_) => {
                let x = self.transform_to_pow();
                Self::linearize(&mut x.data, atom);
            }
            AtomTree::Mul(_) => {
                let x = self.transform_to_mul();
                Self::linearize(&mut x.data, atom);
            }
            AtomTree::Add(_) => {
                let x = self.transform_to_add();
                Self::linearize(&mut x.data, atom);
            }
        }
    }

    pub fn get_data(&self) -> &[u8] {
        match self {
            OwnedAtom::Num(n) => &n.data,
            OwnedAtom::Var(v) => &v.data,
            OwnedAtom::Fun(f) => &f.data,
            OwnedAtom::Pow(p) => &p.data,
            OwnedAtom::Mul(m) => &m.data,
            OwnedAtom::Add(a) => &a.data,
            OwnedAtom::Empty => unreachable!(),
        }
    }

    pub fn len(&self) -> usize {
        self.get_data().len()
    }

    pub fn to_tree(&self) -> AtomTree {
        Self::write_to_tree(self.get_data()).0
    }

    fn linearize(data: &mut Vec<u8>, atom: &AtomTree) {
        match atom {
            AtomTree::Var(name) => {
                data.put_u8(VAR_ID);
                (name.to_u32() as u64, 1).write_packed(data);
            }
            AtomTree::Fn(name, args) => {
                data.put_u8(FUN_ID);
                let size_pos = data.len();
                data.put_u32_le(0 as u32); // length of entire fn without flag
                let buf_pos = data.len();

                // pack name and args
                (name.to_u32() as u64, args.len() as u64).write_packed(data);

                for a in args {
                    Self::linearize(data, a);
                }
                let new_buf_pos = data.len();

                let mut cursor: Cursor<&mut [u8]> = Cursor::new(&mut data[size_pos..]);

                cursor
                    .write_u32::<LittleEndian>((new_buf_pos - buf_pos) as u32)
                    .unwrap();
            }
            AtomTree::Num(n) => {
                data.put_u8(NUM_ID);
                n.clone().write_packed(data);
            }
            AtomTree::Pow(p) => {
                data.put_u8(POW_ID);
                Self::linearize(data, &p.0);
                Self::linearize(data, &p.1);
            }
            AtomTree::Mul(args) | AtomTree::Add(args) => {
                if let AtomTree::Mul(_) = atom {
                    data.put_u8(MUL_ID);
                } else {
                    data.put_u8(ADD_ID);
                }

                let size_pos = data.len();
                data.put_u32_le(0 as u32); // length of entire fn without flag
                let buf_pos = data.len();

                (args.len() as u64, 1).write_packed(data);

                for a in args {
                    Self::linearize(data, a);
                }
                let new_buf_pos = data.len();

                let mut cursor: Cursor<&mut [u8]> = Cursor::new(&mut data[size_pos..]);

                cursor
                    .write_u32::<LittleEndian>((new_buf_pos - buf_pos) as u32)
                    .unwrap();
            }
        }
    }

    fn write_to_tree(mut source: &[u8]) -> (AtomTree, &[u8]) {
        let d = source.get_u8() & TYPE_MASK;
        match d {
            VAR_ID => {
                let name;
                (name, _, source) = source.get_frac_i64();
                (AtomTree::Var(Identifier::from(name as u32)), source)
            }
            FUN_ID => {
                source.get_u32_le(); // size

                let (name, n_args);
                (name, n_args, source) = source.get_frac_i64();

                let mut args = Vec::with_capacity(n_args as usize);
                for _ in 0..n_args {
                    let (a, s) = Self::write_to_tree(source);
                    source = s;
                    args.push(a);
                }

                (AtomTree::Fn(Identifier::from(name as u32), args), source)
            }
            NUM_ID => {
                let n;
                (n, source) = source.get_number_view();
                (AtomTree::Num(n.to_owned()), source)
            }
            POW_ID => {
                let (base, exp);
                (base, source) = Self::write_to_tree(source);
                (exp, source) = Self::write_to_tree(source);
                (AtomTree::Pow(Box::new((base, exp))), source)
            }
            MUL_ID | ADD_ID => {
                source.get_u32_le(); // size

                let n_args;
                (n_args, _, source) = source.get_frac_i64();

                let mut args = Vec::with_capacity(n_args as usize);
                for _ in 0..n_args {
                    let (a, s) = Self::write_to_tree(source);
                    source = s;
                    args.push(a);
                }

                if d == MUL_ID {
                    (AtomTree::Mul(args), source)
                } else {
                    (AtomTree::Add(args), source)
                }
            }
            x => unreachable!("Bad id: {}", x),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq)]
pub struct VarViewD<'a> {
    pub data: &'a [u8],
}

impl<'a, 'b> PartialEq<VarViewD<'b>> for VarViewD<'a> {
    fn eq(&self, other: &VarViewD<'b>) -> bool {
        self.data == other.data
    }
}

#[derive(Debug, Copy, Clone, Eq)]
pub struct FnViewD<'a> {
    pub data: &'a [u8],
}

impl<'a, 'b> PartialEq<FnViewD<'b>> for FnViewD<'a> {
    fn eq(&self, other: &FnViewD<'b>) -> bool {
        self.data == other.data
    }
}

impl<'a> Fun<'a> for FnViewD<'a> {
    type P = DefaultRepresentation;
    type I = ListIteratorD<'a>;

    fn get_name(&self) -> Identifier {
        Identifier::from((&self.data[1 + 4..]).get_frac_i64().0 as u32)
    }

    fn get_nargs(&self) -> usize {
        (&self.data[1 + 4..]).get_frac_i64().1 as usize
    }

    fn is_dirty(&self) -> bool {
        (self.data[0] & DIRTY_FLAG) != 0
    }

    fn cmp(&self, other: &Self) -> Ordering {
        self.get_name().cmp(&other.get_name())
    }

    #[inline]
    fn into_iter(&self) -> Self::I {
        let mut c = self.data;
        c.get_u8();
        c.get_u32_le(); // size

        let n_args;
        (_, n_args, c) = c.get_frac_i64(); // name

        ListIteratorD {
            data: c,
            length: n_args as u32,
        }
    }

    fn to_view(&self) -> AtomView<'a, Self::P> {
        AtomView::Fun(self.clone())
    }
}

#[derive(Debug, Copy, Clone, Eq)]
pub struct NumViewD<'a> {
    pub data: &'a [u8],
}

impl<'a, 'b> PartialEq<NumViewD<'b>> for NumViewD<'a> {
    fn eq(&self, other: &NumViewD<'b>) -> bool {
        self.data == other.data
    }
}

impl<'a> Num<'a> for NumViewD<'a> {
    type P = DefaultRepresentation;

    fn is_zero(&self) -> bool {
        self.data.is_zero_rat()
    }

    fn is_one(&self) -> bool {
        self.data.is_one_rat()
    }

    #[inline]
    fn get_number_view(&self) -> BorrowedNumber<'_> {
        self.data[1..].get_number_view().0
    }

    fn to_view(&self) -> AtomView<'a, Self::P> {
        AtomView::Num(self.clone())
    }
}

#[derive(Debug, Copy, Clone, Eq)]
pub struct PowViewD<'a> {
    pub data: &'a [u8],
}

impl<'a, 'b> PartialEq<PowViewD<'b>> for PowViewD<'a> {
    fn eq(&self, other: &PowViewD<'b>) -> bool {
        self.data == other.data
    }
}

impl<'a> Pow<'a> for PowViewD<'a> {
    type P = DefaultRepresentation;

    #[inline]
    fn get_base(&self) -> AtomView<'a, Self::P> {
        let (b, _) = self.get_base_exp();
        b
    }

    #[inline]
    fn get_exp(&self) -> AtomView<'a, Self::P> {
        let (_, e) = self.get_base_exp();
        e
    }

    #[inline]
    fn get_base_exp(&self) -> (AtomView<'a, Self::P>, AtomView<'a, Self::P>) {
        let mut it = ListIteratorD {
            data: &self.data[1..],
            length: 2,
        };

        (it.next().unwrap(), it.next().unwrap())
    }

    fn to_view(&self) -> AtomView<'a, Self::P> {
        AtomView::Pow(self.clone())
    }
}

#[derive(Debug, Copy, Clone, Eq)]
pub struct MulViewD<'a> {
    pub data: &'a [u8],
}

impl<'a, 'b> PartialEq<MulViewD<'b>> for MulViewD<'a> {
    fn eq(&self, other: &MulViewD<'b>) -> bool {
        self.data == other.data
    }
}

impl<'a> Mul<'a> for MulViewD<'a> {
    type P = DefaultRepresentation;
    type I = ListIteratorD<'a>;

    #[inline]
    fn into_iter(&self) -> Self::I {
        let mut c = self.data;
        c.get_u8();
        c.get_u32_le(); // size

        let n_args;
        (n_args, _, c) = c.get_frac_i64();

        ListIteratorD {
            data: c,
            length: n_args as u32,
        }
    }

    fn get_nargs(&self) -> usize {
        (&self.data[1 + 4..]).get_frac_i64().0 as usize
    }

    fn to_view(&self) -> AtomView<'a, Self::P> {
        AtomView::Mul(self.clone())
    }
}

#[derive(Debug, Copy, Clone, Eq)]
pub struct AddViewD<'a> {
    pub data: &'a [u8],
}

impl<'a, 'b> PartialEq<AddViewD<'b>> for AddViewD<'a> {
    fn eq(&self, other: &AddViewD<'b>) -> bool {
        self.data == other.data
    }
}

impl<'a> Add<'a> for AddViewD<'a> {
    type P = DefaultRepresentation;
    type I = ListIteratorD<'a>;

    #[inline]
    fn into_iter(&self) -> Self::I {
        let mut c = self.data;
        c.get_u8();
        c.get_u32_le(); // size

        let n_args;
        (n_args, _, c) = c.get_frac_i64();

        ListIteratorD {
            data: c,
            length: n_args as u32,
        }
    }

    fn get_nargs(&self) -> usize {
        (&self.data[1 + 4..]).get_frac_i64().0 as usize
    }

    fn to_view(&self) -> AtomView<'a, Self::P> {
        AtomView::Add(self.clone())
    }
}

impl<'a> AtomView<'a, DefaultRepresentation> {
    pub fn from(source: &'a [u8]) -> AtomView<'a, DefaultRepresentation> {
        match source[0] {
            VAR_ID => AtomView::Var(VarViewD { data: source }),
            FUN_ID => AtomView::Fun(FnViewD { data: source }),
            NUM_ID => AtomView::Num(NumViewD { data: source }),
            POW_ID => AtomView::Pow(PowViewD { data: source }),
            MUL_ID => AtomView::Mul(MulViewD { data: source }),
            ADD_ID => AtomView::Add(AddViewD { data: source }),
            x => unreachable!("Bad id: {}", x),
        }
    }

    pub fn get_data(&self) -> &[u8] {
        match self {
            AtomView::Num(n) => n.data,
            AtomView::Var(v) => v.data,
            AtomView::Fun(f) => f.data,
            AtomView::Pow(p) => p.data,
            AtomView::Mul(t) => t.data,
            AtomView::Add(e) => e.data,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ListIteratorD<'a> {
    data: &'a [u8],
    length: u32,
}

impl<'a> ListIterator<'a> for ListIteratorD<'a> {
    type P = DefaultRepresentation;

    #[inline(always)]
    fn next(&mut self) -> Option<AtomView<'a, Self::P>> {
        if self.length == 0 {
            return None;
        }

        self.length -= 1;

        let start = self.data;

        let start_id = self.data.get_u8() & TYPE_MASK;
        let mut cur_id = start_id;

        // store how many more atoms to read
        // can be used instead of storing the byte length of an atom
        let mut skip_count = 1;
        loop {
            match cur_id {
                VAR_ID => {
                    self.data = self.data.skip_rational();
                }
                NUM_ID => {
                    self.data = self.data.skip_rational();
                }
                FUN_ID => {
                    let n_size = self.data.get_u32_le();
                    self.data.advance(n_size as usize);
                }
                POW_ID => {
                    skip_count += 2;
                }
                MUL_ID | ADD_ID => {
                    let n_size = self.data.get_u32_le();
                    self.data.advance(n_size as usize);
                }
                x => unreachable!("Bad id {}", x),
            }

            skip_count -= 1;

            if skip_count == 0 {
                break;
            } else {
                cur_id = self.data.get_u8() & TYPE_MASK;
            }
        }

        let len = unsafe { self.data.as_ptr().offset_from(start.as_ptr()) } as usize;

        let data = unsafe { start.get_unchecked(..len) };
        match start_id {
            VAR_ID => {
                return Some(AtomView::Var(VarViewD { data }));
            }
            NUM_ID => {
                return Some(AtomView::Num(NumViewD { data }));
            }
            FUN_ID => {
                return Some(AtomView::Fun(FnViewD { data }));
            }
            POW_ID => {
                return Some(AtomView::Pow(PowViewD { data }));
            }
            MUL_ID => {
                return Some(AtomView::Mul(MulViewD { data }));
            }
            ADD_ID => {
                return Some(AtomView::Add(AddViewD { data }));
            }
            x => unreachable!("Bad id {}", x),
        }
    }
}

#[test]
pub fn representation_size() {
    let a = AtomTree::Fn(
        Identifier::from(1),
        vec![
            AtomTree::Var(Identifier::from(2)),
            AtomTree::Fn(
                Identifier::from(3),
                vec![
                    AtomTree::Mul(vec![
                        AtomTree::Num(Number::Natural(3, 1)),
                        AtomTree::Num(Number::Natural(13, 1)),
                    ]),
                    AtomTree::Add(vec![
                        AtomTree::Num(Number::Natural(3, 1)),
                        AtomTree::Num(Number::Natural(13, 1)),
                    ]),
                    AtomTree::Mul(vec![
                        AtomTree::Num(Number::Natural(3, 1)),
                        AtomTree::Num(Number::Natural(13, 1)),
                    ]),
                    AtomTree::Mul(vec![
                        AtomTree::Num(Number::Natural(3, 1)),
                        AtomTree::Num(Number::Natural(13, 1)),
                    ]),
                    AtomTree::Num(Number::Natural(4, 2)),
                    AtomTree::Num(Number::Natural(4, 2)),
                    AtomTree::Num(Number::Natural(4, 2)),
                    AtomTree::Num(Number::Natural(4, 2)),
                ],
            ),
            AtomTree::Var(Identifier::from(6)),
            AtomTree::Num(Number::Natural(2, 1)),
            AtomTree::Pow(Box::new((
                AtomTree::Add(vec![
                    AtomTree::Num(Number::Natural(3, 1)),
                    AtomTree::Num(Number::Natural(13, 1)),
                ]),
                AtomTree::Var(Identifier::from(2)),
            ))),
        ],
    );
    println!("expr={:?}", a);

    let mut b = OwnedAtom::new();
    b.from_tree(&a);

    println!("lin size: {:?} bytes", b.get_data().len());

    let c = b.to_tree();

    if a != c {
        panic!("in and out is different: {:?} vs {:?}", a, c);
    }

    b.to_view().print();
}
