use ahash::HashMap;
use dyn_clone::DynClone;

use crate::{
    combinatorics::{partitions, unique_permutations},
    id::{Pattern, PatternRestriction},
    representations::{
        Add, Atom, AtomSet, AtomView, Fun, Identifier, Mul, OwnedAdd, OwnedFun, OwnedMul,
    },
    state::{State, Workspace, ARG},
};

pub trait Map<P: AtomSet>: Fn(AtomView<P>, &mut Atom<P>) + DynClone + Send + Sync {}
dyn_clone::clone_trait_object!(<P: AtomSet> Map<P>);
impl<P: AtomSet, T: Clone + Send + Sync + Fn(AtomView<'_, P>, &mut Atom<P>)> Map<P> for T {}

/// Operations that take a pattern as the input and produce an expression
#[derive(Clone)]
pub enum Transformer<P: AtomSet + 'static> {
    /// Expand the rhs.
    Expand,
    /// Derive the rhs w.r.t a variable.
    Derivative(Identifier),
    /// Derive the rhs w.r.t a variable.
    TaylorSeries(Identifier, Atom<P>, u32),
    /// Apply find-and-replace on the rhs.
    ReplaceAll(
        Pattern<P>,
        Pattern<P>,
        HashMap<Identifier, Vec<PatternRestriction<P>>>,
    ),
    /// Take the product of a list of arguments in the rhs.
    Product,
    /// Take the sum of a list of arguments in the rhs.
    Sum,
    /// Map the rhs with a user-specified function.
    Map(Box<dyn Map<P>>),
    /// Split a `Mul` or `Add` into a list of arguments.
    Split,
    Partition(Vec<(Identifier, usize)>, bool, bool),
    Sort,
    Deduplicate,
    Permutations(Identifier),
    Repeat(Vec<Transformer<P>>),
}

impl<P: AtomSet> std::fmt::Debug for Transformer<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Transformer::Expand => f.debug_tuple("Expand").finish(),
            Transformer::Derivative(x) => f.debug_tuple("Derivative").field(x).finish(),
            Transformer::ReplaceAll(pat, rhs, ..) => {
                f.debug_tuple("ReplaceAll").field(pat).field(rhs).finish()
            }
            Transformer::Product => f.debug_tuple("Product").finish(),
            Transformer::Sum => f.debug_tuple("Sum").finish(),
            Transformer::Map(_) => f.debug_tuple("Map").finish(),
            Transformer::Split => f.debug_tuple("Split").finish(),
            Transformer::Partition(g, b1, b2) => f
                .debug_tuple("Partition")
                .field(g)
                .field(b1)
                .field(b2)
                .finish(),
            Transformer::Sort => f.debug_tuple("Sort").finish(),
            Transformer::Deduplicate => f.debug_tuple("Deduplicate").finish(),
            Transformer::Permutations(i) => f.debug_tuple("Permutations").field(i).finish(),
            Transformer::TaylorSeries(x, point, d) => f
                .debug_tuple("TaylorSeries")
                .field(x)
                .field(point)
                .field(d)
                .finish(),
            Transformer::Repeat(r) => f.debug_tuple("Repeat").field(r).finish(),
        }
    }
}

impl<P: AtomSet> Transformer<P> {
    /// Create a new partition transformer that must exactly fit the input.
    pub fn new_partition_exact(partitions: Vec<(Identifier, usize)>) -> Transformer<P> {
        Transformer::Partition(partitions, false, false)
    }

    /// Create a new partition transformer that collects all left-over
    /// atoms in the last bin.
    pub fn new_partition_collect_in_last(
        mut partitions: Vec<(Identifier, usize)>,
        rest: Identifier,
    ) -> Transformer<P> {
        partitions.push((rest, 0));
        Transformer::Partition(partitions, true, false)
    }

    /// Create a new partition transformer that repeats the partitions so that it can fit
    /// the input.
    pub fn new_partition_repeat(partition: (Identifier, usize)) -> Transformer<P> {
        Transformer::Partition(vec![partition], false, true)
    }

    pub fn execute(
        orig_input: AtomView<'_, P>,
        chain: &[Transformer<P>],
        state: &State,
        workspace: &Workspace<P>,
        out: &mut Atom<P>,
    ) {
        out.set_from_view(&orig_input);
        let mut tmp = workspace.new_atom();
        for t in chain {
            std::mem::swap(out, tmp.get_mut());
            let input = tmp.as_view();

            match t {
                Transformer::Map(f) => {
                    f(input, out);
                }
                Transformer::Expand => {
                    input.expand(workspace, state, out);
                }
                Transformer::Derivative(x) => {
                    input.derivative(*x, workspace, state, out);
                }
                Transformer::TaylorSeries(x, expansion_point, depth) => {
                    input.taylor_series(
                        *x,
                        expansion_point.as_view(),
                        *depth,
                        workspace,
                        state,
                        out,
                    );
                }
                Transformer::ReplaceAll(pat, rhs, cond) => {
                    pat.replace_all(input, &rhs, state, workspace, &cond, out);
                }
                Transformer::Product => {
                    if let AtomView::Fun(f) = input {
                        if f.get_name() == ARG {
                            let mut mul_h = workspace.new_atom();
                            let mul = mul_h.to_mul();

                            for arg in f.iter() {
                                mul.extend(arg);
                            }

                            mul.set_dirty(true);
                            mul_h.as_view().normalize(workspace, state, out);
                            continue;
                        }
                    }

                    out.set_from_view(&input);
                }
                Transformer::Sum => {
                    if let AtomView::Fun(f) = input {
                        if f.get_name() == ARG {
                            let mut add_h = workspace.new_atom();
                            let add = add_h.to_add();

                            for arg in f.iter() {
                                add.extend(arg);
                            }

                            add.set_dirty(true);
                            add_h.as_view().normalize(workspace, state, out);
                            continue;
                        }
                    }

                    out.set_from_view(&input);
                }
                Transformer::Split => match input {
                    AtomView::Mul(m) => {
                        let mut arg_h = workspace.new_atom();
                        let arg = arg_h.to_fun();
                        arg.set_from_name(ARG);

                        for factor in m.iter() {
                            arg.add_arg(factor);
                        }

                        arg.set_dirty(true);
                        arg_h.as_view().normalize(workspace, state, out);
                        continue;
                    }
                    AtomView::Add(a) => {
                        let mut arg_h = workspace.new_atom();
                        let arg = arg_h.to_fun();
                        arg.set_from_name(ARG);

                        for summand in a.iter() {
                            arg.add_arg(summand);
                        }

                        arg.set_dirty(true);
                        arg_h.as_view().normalize(workspace, state, out);
                        continue;
                    }
                    _ => {
                        out.set_from_view(&input);
                    }
                },
                Transformer::Partition(bins, fill_last, repeat) => {
                    if let AtomView::Fun(f) = input {
                        if f.get_name() == ARG {
                            let args: Vec<_> = f.iter().collect();

                            let mut sum_h = workspace.new_atom();
                            let sum = sum_h.to_add();

                            let partitions = partitions(&args, &bins, *fill_last, *repeat);

                            if partitions.is_empty() {
                                out.set_from_view(&workspace.new_num(0).as_view());
                                continue;
                            }

                            for (p, args) in partitions {
                                let mut mul_h = workspace.new_atom();
                                let mul = mul_h.to_mul();

                                if !p.is_one() {
                                    mul.extend(workspace.new_num(p).as_view());
                                }

                                for (name, f_args) in args {
                                    let mut fun_h = workspace.new_atom();
                                    let fun = fun_h.to_fun();
                                    fun.set_from_name(name);
                                    for x in f_args {
                                        fun.add_arg(x);
                                    }
                                    fun.set_dirty(true);

                                    mul.extend(fun_h.as_view());
                                }

                                mul.set_dirty(true);
                                sum.extend(mul_h.as_view());
                            }

                            sum.set_dirty(true);
                            sum_h.as_view().normalize(workspace, state, out);
                            continue;
                        }
                    }

                    out.set_from_view(&input);
                }
                Transformer::Sort => {
                    if let AtomView::Fun(f) = input {
                        if f.get_name() == ARG {
                            let mut args: Vec<_> = f.iter().collect();
                            args.sort();

                            let mut fun_h = workspace.new_atom();
                            let fun = fun_h.to_fun();
                            fun.set_from_name(ARG);

                            for arg in args {
                                fun.add_arg(arg);
                            }

                            fun.set_dirty(true);
                            fun_h.as_view().normalize(workspace, state, out);
                            continue;
                        }
                    }

                    out.set_from_view(&input);
                }
                Transformer::Deduplicate => {
                    if let AtomView::Fun(f) = input {
                        if f.get_name() == ARG {
                            let args: Vec<_> = f.iter().collect();
                            let mut args_dedup: Vec<_> = Vec::with_capacity(args.len());

                            for a in args {
                                // check last argument first, so that the sorted list case is fast
                                if args_dedup.last() != Some(&a) {
                                    if !args_dedup.contains(&a) {
                                        args_dedup.push(a);
                                    }
                                }
                            }

                            let mut fun_h = workspace.new_atom();
                            let fun = fun_h.to_fun();
                            fun.set_from_name(ARG);

                            for arg in args_dedup {
                                fun.add_arg(arg);
                            }

                            fun.set_dirty(true);
                            fun_h.as_view().normalize(workspace, state, out);
                            continue;
                        }
                    }

                    out.set_from_view(&input);
                }
                Transformer::Permutations(f_name) => {
                    if let AtomView::Fun(f) = input {
                        if f.get_name() == ARG {
                            let args: Vec<_> = f.iter().collect();

                            let mut sum_h = workspace.new_atom();
                            let sum = sum_h.to_add();

                            let (prefactor, permutations) = unique_permutations(&args);

                            if permutations.is_empty() {
                                out.set_from_view(&workspace.new_num(0).as_view());
                                continue;
                            }

                            for a in permutations {
                                let mut fun_h = workspace.new_atom();
                                let fun = fun_h.to_fun();
                                fun.set_from_name(*f_name);
                                for x in a {
                                    fun.add_arg(x);
                                }
                                fun.set_dirty(true);

                                if !prefactor.is_one() {
                                    let mut mul_h = workspace.new_atom();
                                    let mul = mul_h.to_mul();
                                    mul.extend(fun_h.as_view());
                                    mul.extend(workspace.new_num(prefactor.clone()).as_view());
                                    mul.set_dirty(true);
                                    sum.extend(mul_h.as_view());
                                } else {
                                    sum.extend(fun_h.as_view());
                                }
                            }

                            sum.set_dirty(true);
                            sum_h.as_view().normalize(workspace, state, out);
                            continue;
                        }
                    }

                    out.set_from_view(&input);
                }
                Transformer::Repeat(r) => loop {
                    Self::execute(tmp.as_view(), &r, state, workspace, out);

                    if tmp.as_view() == out.as_view() {
                        break;
                    }

                    std::mem::swap(out, &mut tmp);
                },
            }
        }
    }
}
