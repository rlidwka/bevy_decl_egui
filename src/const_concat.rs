// this heinous hackery is released under Unlicense
// https://github.com/eira-fransham/const-concat

pub const unsafe fn transmute<From, To>(from: From) -> To {
    union Transmute<From, To> {
        from: std::mem::ManuallyDrop<From>,
        to: std::mem::ManuallyDrop<To>,
    }

    std::mem::ManuallyDrop::into_inner(Transmute { from: std::mem::ManuallyDrop::new(from) }.to)
}

pub const unsafe fn concat<First, Second, Out>(a: &'static [&'static str], b: &'static [&'static str]) -> Out
where
    First: Copy,
    Second: Copy,
    Out: Copy,
{
    #[repr(C)]
    #[derive(Copy, Clone)]
    struct Both<A, B>(A, B);

    let arr: Both<First, Second> = Both(
        *transmute::<_, *const First>(a.as_ptr()),
        *transmute::<_, *const Second>(b.as_ptr()),
    );

    transmute(arr)
}

#[macro_export]
macro_rules! const_concat {
    () => {
        []
    };
    ($a:expr) => {
        $a
    };
    ($a:expr, $b:expr) => {{
        let ret: &'static [&'static str] = unsafe {
            &$crate::const_concat::concat::<
                [&'static str; $a.len()],
                [&'static str; $b.len()],
                [&'static str; $a.len() + $b.len()],
            >($a, $b)
        };

        unsafe { $crate::const_concat::transmute::<_, &'static [&'static str]>(ret) }
    }};
    ($a:expr, $($rest:expr),*) => {{
        const TAIL: &'static [&'static str] = const_concat!($($rest),*);
        const_concat!($a, TAIL)
    }};
    ($a:expr, $($rest:expr),*,) => {
        const_concat!($a, $($rest),*)
    };
}
