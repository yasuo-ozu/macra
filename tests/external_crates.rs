use std::path::PathBuf;

use cargo_macra::parse_trace::{MacroExpansion, MacroExpansionKind};
use cargo_macra::trace_macros::{Args as TraceArgs, TraceMacros};

fn repo_manifest(repo: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/external_crates").join(repo).join("Cargo.toml")
}

fn run_trace_for_repo(repo: &str, test: Option<&str>) -> Vec<MacroExpansion> {
    let manifest_path = repo_manifest(repo);
    assert!(manifest_path.exists());
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let tm_args = TraceArgs {
        package: None, bin: None, lib: false, test: test.map(|s| s.to_string()), example: None,
        manifest_path: Some(manifest_path.to_string_lossy().to_string()), cargo_args: Vec::new(),
        hook_lib: cargo_macra::find_hook_lib(std::env::current_exe().ok().as_deref()).unwrap_or_default(),
    };
    let tm = TraceMacros::new(std::path::Path::new(&cargo), &tm_args);
    let run = tm.run().unwrap_or_else(|e| panic!("TraceMacros::run() failed for {}: {}", repo, e));
    let expansions = run.iter.collect::<std::io::Result<Vec<_>>>().unwrap_or_else(|e| panic!("failed to collect expansions for {}: {}", repo, e));
    let check_success = run.check_success.recv().expect("failed to receive cargo check status").unwrap_or_else(|e| panic!("failed waiting cargo check for {}: {}", repo, e));
    assert!(check_success);
    assert!(!expansions.is_empty());
    expansions
}

#[test]
fn external_crate_coinduction_show_expansion() {
    let expansions = run_trace_for_repo("coinduction", None);
}

#[test]
fn external_crate_coinduction_test_coinduction_integration_test() {
    let expansions = run_trace_for_repo("coinduction", Some("coinduction_integration_test"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "traitdef"
            && e.input == r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#
            && e.to == r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_3044150873991545574
{
    ("0.2.0", None, [[$T:ty; $N:expr] :$ ($wt : tt) *],
    { $ ($coinduction : tt) + }, $ ($t : tt) *) =>
    {
        $ ($coinduction) +:: __next_step!
        {
            "0.2.0", Traitdef
            {
                appending_constraints :
                [$T : :: core :: clone :: Clone + :: core :: default ::
                Default]
            }, [[$T; $N] :$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
        }
    };
    ("0.2.0", None, [[$T:ty] :$ ($wt : tt) *], { $ ($coinduction : tt) + }, $
    ($t : tt) *) =>
    {
        $ ($coinduction) +:: __next_step!
        {
            "0.2.0", Traitdef
            {
                appending_constraints :
                [$T : :: core :: cmp :: PartialEq + :: core :: clone :: Clone]
            }, [[$T] :$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
        }
    };
    ("0.2.0", None, [($T:ty, $U:ty) :$ ($wt : tt) *],
    { $ ($coinduction : tt) + }, $ ($t : tt) *) =>
    {
        $ ($coinduction) +:: __next_step!
        {
            "0.2.0", Traitdef
            {
                appending_constraints :
                [$T : :: core :: clone :: Clone + :: core :: marker :: Send,
                $U : :: core :: marker :: Sync + :: core :: default ::
                Default]
            }, [($T, $U) :$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
        }
    };
    ("0.2.0", None, [($T:ty, $U:ty, $V:ty) :$ ($wt : tt) *],
    { $ ($coinduction : tt) + }, $ ($t : tt) *) =>
    {
        $ ($coinduction) +:: __next_step!
        {
            "0.2.0", Traitdef
            {
                appending_constraints :
                [$T : :: core :: clone :: Clone + :: core :: marker :: Send,
                $U : :: core :: marker :: Sync, $V : :: core :: default ::
                Default + :: core :: marker :: Send]
            }, [($T, $U, $V) :$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
        }
    };
    ("0.2.0", None,
    [:: $seg0 : ident $ (:: $segs : ident) * $ (<$ ($arg : ty),*$ (,) ?>) ? :$
    ($wt : tt) *], { $ ($coinduction : tt) + }, $ ($t : tt) *) =>
    {
        :: $seg0 $ (:: $segs) * !
        {
            "0.2.0", None,
            [$ty0 : :: $seg0 $ (:: $segs) * $ (<$ ($arg),*>) ? :$ ($wt) *],
            { $ ($coinduction) + }, $ ($t) *
        }
    };
    ("0.2.0", None,
    [$seg0 : ident $ (:: $segs : ident) * $ (<$ ($arg : ty),*$ (,) ?>) ? :$
    ($wt : tt) *], { $ ($coinduction : tt) + }, $ ($t : tt) *) =>
    {
        $seg0 $ (:: $segs) *!
        {
            "0.2.0", None,
            [$seg0 $ (:: $segs) * $ (<$ ($arg),*>) ? :$ ($wt) *],
            { $ ($coinduction) + }, $ ($t) *
        }
    };
} #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub use
__LocalTrait_temporal_3044150873991545574 as LocalTrait;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "typedef"
            && e.input == r#"pub mod generic_types
{
    use super::*; use std::fmt::Debug; use std::hash::Hash; pub struct
    Container<T, U> { pub first: T, pub second: U, } pub struct Wrapper<T>
    where T: Clone + Debug, { pub value: T, pub count: usize, } pub struct
    MultiGeneric<T, U, V> where T: Clone, U: Send + Sync, V: Debug + Hash,
    { pub primary: T, pub secondary: U, pub metadata: V, } pub struct
    ConstrainedStruct<T> where T: Iterator + Clone, { pub iterator: T, }
    impl<T, U> TestTrait for Container<T, U> where T: Clone +
    ::std::fmt::Debug + Send, U: ::std::fmt::Debug + Default + Sync,
    { fn test_method(&self) -> String { format!("{:?}", self.first) } }
    impl<T> TestTrait for Wrapper<T> where T: Clone + Debug + ToString,
    {
        fn test_method(&self) -> String
        { format!("{}: {}", self.value.to_string(), self.count) }
    } impl<T, U> LocalTrait for Container<T, U> where T: Clone + Send + Sync,
    U: ::std::fmt::Debug + Hash,
    { fn local_method(&self) -> usize { let _ = self.first.clone(); 42 } }
    impl<T> LocalTrait for Wrapper<T> where T: Clone + ::std::fmt::Debug +
    Default,
    { fn local_method(&self) -> usize { let _ = T::default(); self.count } }
    impl<T, U, V> CircularTrait for MultiGeneric<T, U, V> where T: Clone +
    ::std::fmt::Debug + Send + 'static, U: Send + Sync + Default, V:
    ::std::fmt::Debug + Hash + Clone,
    {
        fn circular_method(&self) -> Box<dyn CircularTrait>
        {
            Box::new(ConstrainedStruct
            { iterator: std::iter::once(self.primary.clone()), })
        }
    } impl<T> CircularTrait for ConstrainedStruct<T> where T: Iterator + Clone
    + Send, T::Item: ::std::fmt::Debug,
    {
        fn circular_method(&self) -> Box<dyn CircularTrait>
        {
            Box::new(MultiGeneric
            {
                primary: "circular".to_string(), secondary: 42u32, metadata:
                123usize,
            })
        }
    } impl<T, U> ExtendedTrait for Container<T, U> where T: PartialEq + Clone,
    U: Default + Send,
    {
        fn extended_method(&self) -> bool
        { let _default_u = U::default(); true }
    } impl<T, U, V> ExtendedTrait for MultiGeneric<T, U, V> where T: Clone +
    PartialOrd, U: Send + Sync + Clone, V: ::std::fmt::Debug + Hash + Default,
    {
        fn extended_method(&self) -> bool
        { let _ = V::default(); let _ = self.secondary.clone(); true }
    }
}"#
            && e.to == r#"pub mod generic_types
{
    use super :: * ; use std :: fmt :: Debug; use std :: hash :: Hash; pub
    struct Container < T, U > { pub first : T, pub second : U, } pub struct
    Wrapper < T > where T : Clone + Debug,
    { pub value : T, pub count : usize, } pub struct MultiGeneric < T, U, V >
    where T : Clone, U : Send + Sync, V : Debug + Hash,
    { pub primary : T, pub secondary : U, pub metadata : V, } pub struct
    ConstrainedStruct < T > where T : Iterator + Clone, { pub iterator : T, }
    impl < T, U > TestTrait for Container < T, U > where T : Clone + :: std ::
    fmt :: Debug + Send, U : :: std :: fmt :: Debug + Default + Sync,
    { fn test_method(& self) -> String { format! ("{:?}", self.first) } } impl
    < T > TestTrait for Wrapper < T > where T : Clone + Debug + ToString,
    {
        fn test_method(& self) -> String
        { format! ("{}: {}", self.value.to_string(), self.count) }
    } impl < T, U > LocalTrait for Container < T, U > where T : Clone + Send +
    Sync, U : :: std :: fmt :: Debug + Hash,
    { fn local_method(& self) -> usize { let _ = self.first.clone(); 42 } }
    impl < T > LocalTrait for Wrapper < T > where T : Clone + :: std :: fmt ::
    Debug + Default,
    {
        fn local_method(& self) -> usize
        { let _ = T :: default(); self.count }
    } impl < T, U, V > CircularTrait for MultiGeneric < T, U, V > where T :
    Clone + :: std :: fmt :: Debug + Send + 'static, U : Send + Sync +
    Default, V : :: std :: fmt :: Debug + Hash + Clone,
    {
        fn circular_method(& self) -> Box < dyn CircularTrait >
        {
            Box ::
            new(ConstrainedStruct
            { iterator : std :: iter :: once(self.primary.clone()), })
        }
    } impl < T > CircularTrait for ConstrainedStruct < T > where T : Iterator
    + Clone + Send, T :: Item : :: std :: fmt :: Debug,
    {
        fn circular_method(& self) -> Box < dyn CircularTrait >
        {
            Box ::
            new(MultiGeneric
            {
                primary : "circular".to_string(), secondary : 42u32, metadata
                : 123usize,
            })
        }
    } impl < T, U > ExtendedTrait for Container < T, U > where T : PartialEq +
    Clone, U : Default + Send,
    {
        fn extended_method(& self) -> bool
        { let _default_u = U :: default(); true }
    } impl < T, U, V > ExtendedTrait for MultiGeneric < T, U, V > where T :
    Clone + PartialOrd, U : Send + Sync + Clone, V : :: std :: fmt :: Debug +
    Hash + Default,
    {
        fn extended_method(& self) -> bool
        { let _ = V :: default(); let _ = self.secondary.clone(); true }
    }
    #[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
    #[doc(hidden)] #[macro_export] macro_rules!
    __Container_temporal_13753066654786479059
    {
        ("0.2.0", None, [$ ($wt : tt) *], { $ ($coinduction : tt) + }, $
        ($t : tt) *) =>
        {
            $ ($coinduction) +:: __next_step!
            {
                "0.2.0", Typedef
                {
                    predicates :
                    [([__T_7_13753066654786479059, __U_7_13753066654786479059],
                    Container < __T_7_13753066654786479059,
                    __U_7_13753066654786479059 > : TestTrait,
                    [__T_7_13753066654786479059 : Clone,
                    __T_7_13753066654786479059 : :: std :: fmt :: Debug,
                    __T_7_13753066654786479059 : Send,
                    __U_7_13753066654786479059 : :: std :: fmt :: Debug,
                    __U_7_13753066654786479059 : Default,
                    __U_7_13753066654786479059 : Sync]),
                    ([__T_9_13753066654786479059, __U_9_13753066654786479059],
                    Container < __T_9_13753066654786479059,
                    __U_9_13753066654786479059 > : LocalTrait,
                    [__T_9_13753066654786479059 : Clone,
                    __T_9_13753066654786479059 : Send,
                    __T_9_13753066654786479059 : Sync,
                    __U_9_13753066654786479059 : :: std :: fmt :: Debug,
                    __U_9_13753066654786479059 : Hash]),
                    ([__T_13_13753066654786479059, __U_13_13753066654786479059],
                    Container < __T_13_13753066654786479059,
                    __U_13_13753066654786479059 > : ExtendedTrait,
                    [__T_13_13753066654786479059 : PartialEq,
                    __T_13_13753066654786479059 : Clone,
                    __U_13_13753066654786479059 : Default,
                    __U_13_13753066654786479059 : Send])]
                }, [$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
            }
        }
    } #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub
    use __Container_temporal_13753066654786479059 as Container;
    #[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
    #[doc(hidden)] #[macro_export] macro_rules!
    __Wrapper_temporal_13753066654786479059
    {
        ("0.2.0", None, [$ ($wt : tt) *], { $ ($coinduction : tt) + }, $
        ($t : tt) *) =>
        {
            $ ($coinduction) +:: __next_step!
            {
                "0.2.0", Typedef
                {
                    predicates :
                    [([__T_8_13753066654786479059], Wrapper <
                    __T_8_13753066654786479059 > : TestTrait,
                    [__T_8_13753066654786479059 : Clone,
                    __T_8_13753066654786479059 : Debug,
                    __T_8_13753066654786479059 : ToString]),
                    ([__T_10_13753066654786479059], Wrapper <
                    __T_10_13753066654786479059 > : LocalTrait,
                    [__T_10_13753066654786479059 : Clone,
                    __T_10_13753066654786479059 : :: std :: fmt :: Debug,
                    __T_10_13753066654786479059 : Default])]
                }, [$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
            }
        }
    } #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub
    use __Wrapper_temporal_13753066654786479059 as Wrapper;
    #[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
    #[doc(hidden)] #[macro_export] macro_rules!
    __MultiGeneric_temporal_13753066654786479059
    {
        ("0.2.0", None, [$ ($wt : tt) *], { $ ($coinduction : tt) + }, $
        ($t : tt) *) =>
        {
            $ ($coinduction) +:: __next_step!
            {
                "0.2.0", Typedef
                {
                    predicates :
                    [([__T_11_13753066654786479059, __U_11_13753066654786479059,
                    __V_11_13753066654786479059], MultiGeneric <
                    __T_11_13753066654786479059, __U_11_13753066654786479059,
                    __V_11_13753066654786479059 > : CircularTrait,
                    [__T_11_13753066654786479059 : Clone,
                    __T_11_13753066654786479059 : :: std :: fmt :: Debug,
                    __T_11_13753066654786479059 : Send,
                    __U_11_13753066654786479059 : Send,
                    __U_11_13753066654786479059 : Sync,
                    __U_11_13753066654786479059 : Default,
                    __V_11_13753066654786479059 : :: std :: fmt :: Debug,
                    __V_11_13753066654786479059 : Hash,
                    __V_11_13753066654786479059 : Clone]),
                    ([__T_14_13753066654786479059, __U_14_13753066654786479059,
                    __V_14_13753066654786479059], MultiGeneric <
                    __T_14_13753066654786479059, __U_14_13753066654786479059,
                    __V_14_13753066654786479059 > : ExtendedTrait,
                    [__T_14_13753066654786479059 : Clone,
                    __T_14_13753066654786479059 : PartialOrd,
                    __U_14_13753066654786479059 : Send,
                    __U_14_13753066654786479059 : Sync,
                    __U_14_13753066654786479059 : Clone,
                    __V_14_13753066654786479059 : :: std :: fmt :: Debug,
                    __V_14_13753066654786479059 : Hash,
                    __V_14_13753066654786479059 : Default])]
                }, [$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
            }
        }
    } #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub
    use __MultiGeneric_temporal_13753066654786479059 as MultiGeneric;
    #[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
    #[doc(hidden)] #[macro_export] macro_rules!
    __ConstrainedStruct_temporal_13753066654786479059
    {
        ("0.2.0", None, [$ ($wt : tt) *], { $ ($coinduction : tt) + }, $
        ($t : tt) *) =>
        {
            $ ($coinduction) +:: __next_step!
            {
                "0.2.0", Typedef
                {
                    predicates :
                    [([__T_12_13753066654786479059], ConstrainedStruct <
                    __T_12_13753066654786479059 > : CircularTrait,
                    [__T_12_13753066654786479059 : Iterator,
                    __T_12_13753066654786479059 : Clone,
                    __T_12_13753066654786479059 : Send, T :: Item : :: std ::
                    fmt :: Debug])]
                }, [$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
            }
        }
    } #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub
    use __ConstrainedStruct_temporal_13753066654786479059 as
    ConstrainedStruct;
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__CircularTrait_temporal_6874738198656642577"
            && e.input == r#""0.2.0", None,
           [typedef :: generic_types :: ConstrainedStruct < std :: iter :: Once < NodeC <
           T > > > : CircularTrait, typedef :: generic_types :: Container < NodeB < T > ,
           NodeC < T > > : LocalTrait, typedef :: generic_types :: MultiGeneric < NodeC <
           T > , NodeB < T > , String > : CircularTrait, typedef :: generic_types ::
           Wrapper < NodeB < T > > : LocalTrait, typedef :: generic_types :: Container <
           NodeB < T > , InternalType > : LocalTrait, typedef :: generic_types :: Wrapper
           < NodeA < T > > : LocalTrait, typedef :: generic_types :: Container < NodeA <
           T > , NodeB < T > > : LocalTrait], { :: coinduction },
           [LocalTrait, TestTrait, CircularTrait, CoinductionLocalTrait],
           [NodeA, NodeB, NodeC],
           [{
               [],
               [(NodeA < T > : CircularTrait, T : Clone),
               (NodeA < T > : CircularTrait, T : Send),
               (NodeA < T > : CircularTrait, NodeA < T > : Clone),
               (NodeA < T > : CircularTrait, typedef :: generic_types :: Container <
               NodeB < T > , InternalType > : LocalTrait),
               (NodeA < T > : CircularTrait, typedef :: generic_types ::
               ConstrainedStruct < std :: iter :: Once < NodeC < T > > > :
               CircularTrait)], [T : 'static + Clone + Send]
           }, None, None, None, None, None, None, None, None, None, None, None, None,
           None, None, None, None, None, None, None,
           {
               [],
               [(NodeA < T > : TestTrait, typedef :: generic_types :: Container < NodeB <
               T > , NodeC < T > > : LocalTrait)], [T]
           }, { [NodeB < T > : LocalTrait], [], [T] },
           {
               [],
               [(NodeB < T > : CircularTrait, T : Clone),
               (NodeB < T > : CircularTrait, T : Send),
               (NodeB < T > : CircularTrait, T : Clone),
               (NodeB < T > : CircularTrait, typedef :: generic_types :: Wrapper < NodeA
               < T > > : LocalTrait),
               (NodeB < T > : CircularTrait, typedef :: generic_types :: MultiGeneric <
               NodeC < T > , NodeB < T > , String > : CircularTrait)],
               [T : 'static + Clone + Send]
           },
           {
               [],
               [(NodeC < T > : CircularTrait, T : Clone),
               (NodeC < T > : CircularTrait, T : Send),
               (NodeC < T > : CircularTrait, T : Clone),
               (NodeC < T > : CircularTrait, NodeA < T > : TestTrait),
               (NodeC < T > : CircularTrait, NodeB < T > : LocalTrait),
               (NodeC < T > : CircularTrait, typedef :: generic_types :: Container <
               NodeA < T > , NodeB < T > > : LocalTrait),
               (NodeA < T > : TestTrait, typedef :: generic_types :: Container < NodeB <
               T > , NodeC < T > > : LocalTrait)], [T : 'static + Clone + Send]
           },
           {
               [],
               [(NodeA < T > : CoinductionLocalTrait, T : Clone),
               (NodeA < T > : CoinductionLocalTrait, typedef :: generic_types :: Wrapper
               < NodeB < T > > : LocalTrait)], [T : Clone]
           }, { [NodeB < T > : CoinductionLocalTrait], [], [T] },
           {
               [],
               [(NodeC < T > : CoinductionLocalTrait, NodeA < T > : TestTrait),
               (NodeC < T > : CoinductionLocalTrait, NodeA < T > :
               CoinductionLocalTrait),
               (NodeC < T > : CoinductionLocalTrait, NodeB < T > : LocalTrait),
               (NodeC < T > : CoinductionLocalTrait, NodeB < T > :
               CoinductionLocalTrait),
               (NodeA < T > : TestTrait, typedef :: generic_types :: Container < NodeB <
               T > , NodeC < T > > : LocalTrait),
               (NodeA < T > : CoinductionLocalTrait, T : Clone),
               (NodeA < T > : CoinductionLocalTrait, typedef :: generic_types :: Wrapper
               < NodeB < T > > : LocalTrait)], [T]
           }],
           [impl < T : 'static + Clone + Send > CircularTrait for NodeA < T > where NodeA
           < T > : Clone, typedef :: generic_types :: Container < NodeB < T > ,
           InternalType > : LocalTrait, typedef :: generic_types :: ConstrainedStruct <
           std :: iter :: Once < NodeC < T > > > : CircularTrait,
           {
               fn circular_method(& self) -> Box < dyn CircularTrait >
               {
                   Box ::
                   new(NodeB :: < T >
                   {
                       count : self.value.len(), child_a : None, internal :
                       InternalType(42.0), phantom : PhantomData,
                   })
               }
           }, impl < T : Clone > Clone for NodeA < T >
           {
               fn clone(& self) -> Self
               {
                   NodeA
                   {
                       value : self.value.clone(), child_b : self.child_b.clone(),
                       phantom : self.phantom,
                   }
               }
           }, impl < T > std :: fmt :: Debug for NodeA < T >
           {
               fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt ::
               Result
               {
                   f.debug_struct("NodeA").field("value", &
                   self.value).field("child_b", & "<child_b>").finish()
               }
           }, impl < T > PartialEq for NodeA < T >
           { fn eq(& self, other : & Self) -> bool { self.value == other.value } }, impl
           < T > ToString for NodeA < T >
           { fn to_string(& self) -> String { format! ("NodeA({})", self.value) } },
           unsafe impl < T > Send for NodeA < T > {}, unsafe impl < T > Sync for NodeA <
           T > {}, impl < T : Clone > Clone for NodeB < T >
           {
               fn clone(& self) -> Self
               {
                   NodeB
                   {
                       count : self.count, child_a : self.child_a.clone(), internal :
                       self.internal.clone(), phantom : self.phantom,
                   }
               }
           }, impl < T > std :: fmt :: Debug for NodeB < T >
           {
               fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt ::
               Result
               {
                   f.debug_struct("NodeB").field("count", &
                   self.count).field("child_a", &
                   "<child_a>").field("internal", & self.internal).finish()
               }
           }, impl < T > PartialEq for NodeB < T >
           {
               fn eq(& self, other : & Self) -> bool
               { self.count == other.count && self.internal == other.internal }
           }, unsafe impl < T > Send for NodeB < T > {}, unsafe impl < T > Sync for NodeB
           < T > {}, impl < T : Clone > Clone for NodeC < T >
           {
               fn clone(& self) -> Self
               {
                   NodeC
                   {
                       data : self.data, ref_a : self.ref_a.clone(), ref_b :
                       self.ref_b.clone(), phantom : self.phantom,
                   }
               }
           }, impl < T > std :: fmt :: Debug for NodeC < T >
           {
               fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt ::
               Result
               {
                   f.debug_struct("NodeC").field("data", &
                   self.data).field("ref_a", &
                   "<ref_a>").field("ref_b", & "<ref_b>").finish()
               }
           }, impl < T > PartialEq for NodeC < T >
           { fn eq(& self, other : & Self) -> bool { self.data == other.data } }, impl <
           T > std :: hash :: Hash for NodeC < T >
           {
               fn hash < H : std :: hash :: Hasher > (& self, state : & mut H)
               { self.data.hash(state); }
           }, impl < T > std :: hash :: Hash for NodeB < T >
           {
               fn hash < H : std :: hash :: Hasher > (& self, state : & mut H)
               { self.count.hash(state); }
           }, impl < T > Default for NodeA < T >
           {
               fn default() -> Self
               {
                   NodeA
                   { value : String :: new(), child_b : None, phantom : PhantomData, }
               }
           }, impl < T > Default for NodeB < T >
           {
               fn default() -> Self
               {
                   NodeB
                   {
                       count : 0, child_a : None, internal : InternalType(0.0), phantom :
                       PhantomData,
                   }
               }
           }, impl < T > Default for NodeC < T >
           {
               fn default() -> Self
               { NodeC { data : 0, ref_a : None, ref_b : None, phantom : PhantomData, } }
           }, impl < T > TestTrait for NodeA < T > where typedef :: generic_types ::
           Container < NodeB < T > , NodeC < T > > : LocalTrait,
           { fn test_method(& self) -> String { format! ("NodeA: {}", self.value) } },
           impl < T > LocalTrait for NodeB < T >
           {
               fn local_method(& self) -> usize
               { self.count + (self.internal.coinduction_method() as usize) }
           }, impl < T : 'static + Clone + Send > CircularTrait for NodeB < T > where T :
           Clone, typedef :: generic_types :: Wrapper < NodeA < T > > : LocalTrait,
           typedef :: generic_types :: MultiGeneric < NodeC < T > , NodeB < T > , String
           > : CircularTrait,
           {
               fn circular_method(& self) -> Box < dyn CircularTrait >
               {
                   Box ::
                   new(NodeC :: < T >
                   {
                       data : self.count as i32, ref_a : None, ref_b : None, phantom :
                       PhantomData,
                   })
               }
           }, impl < T : 'static + Clone + Send > CircularTrait for NodeC < T > where T :
           Clone, NodeA < T > : TestTrait, NodeB < T > : LocalTrait, typedef ::
           generic_types :: Container < NodeA < T > , NodeB < T > > : LocalTrait,
           {
               fn circular_method(& self) -> Box < dyn CircularTrait >
               {
                   Box ::
                   new(NodeA :: < T >
                   {
                       value : format! ("Generated from NodeC: {}", self.data), child_b :
                       None, phantom : PhantomData,
                   })
               }
           }, impl < T : Clone > CoinductionLocalTrait for NodeA < T > where typedef ::
           generic_types :: Wrapper < NodeB < T > > : LocalTrait,
           { fn coinduction_method(& self) -> f64 { self.value.len() as f64 } }, impl < T
           > CoinductionLocalTrait for NodeB < T >
           {
               fn coinduction_method(& self) -> f64
               { self.count as f64 * self.internal.0 }
           }, impl < T > CoinductionLocalTrait for NodeC < T > where NodeA < T > :
           TestTrait + CoinductionLocalTrait, NodeB < T > : LocalTrait +
           CoinductionLocalTrait,
           { fn coinduction_method(& self) -> f64 { self.data as f64 * 2.0 } }]"#
            && e.to == r#"typedef :: generic_types :: ConstrainedStruct !
           {
               "0.2.0", None,
               [typedef :: generic_types :: ConstrainedStruct <
               std :: iter :: Once < NodeC < T > > > : CircularTrait, typedef ::
               generic_types :: Container < NodeB < T > , NodeC < T > > : LocalTrait,
               typedef :: generic_types :: MultiGeneric < NodeC < T > , NodeB < T > ,
               String > : CircularTrait, typedef :: generic_types :: Wrapper < NodeB < T
               > > : LocalTrait, typedef :: generic_types :: Container < NodeB < T > ,
               InternalType > : LocalTrait, typedef :: generic_types :: Wrapper < NodeA <
               T > > : LocalTrait, typedef :: generic_types :: Container < NodeA < T > ,
               NodeB < T > > : LocalTrait], { :: coinduction },
               [LocalTrait, TestTrait, CircularTrait, CoinductionLocalTrait],
               [NodeA, NodeB, NodeC],
               [{
                   [],
                   [(NodeA < T > : CircularTrait, T : Clone),
                   (NodeA < T > : CircularTrait, T : Send),
                   (NodeA < T > : CircularTrait, NodeA < T > : Clone),
                   (NodeA < T > : CircularTrait, typedef :: generic_types :: Container <
                   NodeB < T > , InternalType > : LocalTrait),
                   (NodeA < T > : CircularTrait, typedef :: generic_types ::
                   ConstrainedStruct < std :: iter :: Once < NodeC < T > > > :
                   CircularTrait)], [T : 'static + Clone + Send]
               }, None, None, None, None, None, None, None, None, None, None, None, None,
               None, None, None, None, None, None, None,
               {
                   [],
                   [(NodeA < T > : TestTrait, typedef :: generic_types :: Container <
                   NodeB < T > , NodeC < T > > : LocalTrait)], [T]
               }, { [NodeB < T > : LocalTrait], [], [T] },
               {
                   [],
                   [(NodeB < T > : CircularTrait, T : Clone),
                   (NodeB < T > : CircularTrait, T : Send),
                   (NodeB < T > : CircularTrait, T : Clone),
                   (NodeB < T > : CircularTrait, typedef :: generic_types :: Wrapper <
                   NodeA < T > > : LocalTrait),
                   (NodeB < T > : CircularTrait, typedef :: generic_types :: MultiGeneric
                   < NodeC < T > , NodeB < T > , String > : CircularTrait)],
                   [T : 'static + Clone + Send]
               },
               {
                   [],
                   [(NodeC < T > : CircularTrait, T : Clone),
                   (NodeC < T > : CircularTrait, T : Send),
                   (NodeC < T > : CircularTrait, T : Clone),
                   (NodeC < T > : CircularTrait, NodeA < T > : TestTrait),
                   (NodeC < T > : CircularTrait, NodeB < T > : LocalTrait),
                   (NodeC < T > : CircularTrait, typedef :: generic_types :: Container <
                   NodeA < T > , NodeB < T > > : LocalTrait),
                   (NodeA < T > : TestTrait, typedef :: generic_types :: Container <
                   NodeB < T > , NodeC < T > > : LocalTrait)],
                   [T : 'static + Clone + Send]
               },
               {
                   [],
                   [(NodeA < T > : CoinductionLocalTrait, T : Clone),
                   (NodeA < T > : CoinductionLocalTrait, typedef :: generic_types ::
                   Wrapper < NodeB < T > > : LocalTrait)], [T : Clone]
               }, { [NodeB < T > : CoinductionLocalTrait], [], [T] },
               {
                   [],
                   [(NodeC < T > : CoinductionLocalTrait, NodeA < T > : TestTrait),
                   (NodeC < T > : CoinductionLocalTrait, NodeA < T > :
                   CoinductionLocalTrait),
                   (NodeC < T > : CoinductionLocalTrait, NodeB < T > : LocalTrait),
                   (NodeC < T > : CoinductionLocalTrait, NodeB < T > :
                   CoinductionLocalTrait),
                   (NodeA < T > : TestTrait, typedef :: generic_types :: Container <
                   NodeB < T > , NodeC < T > > : LocalTrait),
                   (NodeA < T > : CoinductionLocalTrait, T : Clone),
                   (NodeA < T > : CoinductionLocalTrait, typedef :: generic_types ::
                   Wrapper < NodeB < T > > : LocalTrait)], [T]
               }],
               [impl < T : 'static + Clone + Send > CircularTrait for NodeA < T > where
               NodeA < T > : Clone, typedef :: generic_types :: Container < NodeB < T > ,
               InternalType > : LocalTrait, typedef :: generic_types :: ConstrainedStruct
               < std :: iter :: Once < NodeC < T > > > : CircularTrait,
               {
                   fn circular_method(& self) -> Box < dyn CircularTrait >
                   {
                       Box ::
                       new(NodeB :: < T >
                       {
                           count : self.value.len(), child_a : None, internal :
                           InternalType(42.0), phantom : PhantomData,
                       })
                   }
               }, impl < T : Clone > Clone for NodeA < T >
               {
                   fn clone(& self) -> Self
                   {
                       NodeA
                       {
                           value : self.value.clone(), child_b : self.child_b.clone(),
                           phantom : self.phantom,
                       }
                   }
               }, impl < T > std :: fmt :: Debug for NodeA < T >
               {
                   fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt
                   :: Result
                   {
                       f.debug_struct("NodeA").field("value", &
                       self.value).field("child_b", & "<child_b>").finish()
                   }
               }, impl < T > PartialEq for NodeA < T >
               { fn eq(& self, other : & Self) -> bool { self.value == other.value } },
               impl < T > ToString for NodeA < T >
               { fn to_string(& self) -> String { format! ("NodeA({})", self.value) } },
               unsafe impl < T > Send for NodeA < T > {}, unsafe impl < T > Sync for
               NodeA < T > {}, impl < T : Clone > Clone for NodeB < T >
               {
                   fn clone(& self) -> Self
                   {
                       NodeB
                       {
                           count : self.count, child_a : self.child_a.clone(), internal :
                           self.internal.clone(), phantom : self.phantom,
                       }
                   }
               }, impl < T > std :: fmt :: Debug for NodeB < T >
               {
                   fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt
                   :: Result
                   {
                       f.debug_struct("NodeB").field("count", &
                       self.count).field("child_a", &
                       "<child_a>").field("internal", & self.internal).finish()
                   }
               }, impl < T > PartialEq for NodeB < T >
               {
                   fn eq(& self, other : & Self) -> bool
                   { self.count == other.count && self.internal == other.internal }
               }, unsafe impl < T > Send for NodeB < T > {}, unsafe impl < T > Sync for
               NodeB < T > {}, impl < T : Clone > Clone for NodeC < T >
               {
                   fn clone(& self) -> Self
                   {
                       NodeC
                       {
                           data : self.data, ref_a : self.ref_a.clone(), ref_b :
                           self.ref_b.clone(), phantom : self.phantom,
                       }
                   }
               }, impl < T > std :: fmt :: Debug for NodeC < T >
               {
                   fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt
                   :: Result
                   {
                       f.debug_struct("NodeC").field("data", &
                       self.data).field("ref_a", &
                       "<ref_a>").field("ref_b", & "<ref_b>").finish()
                   }
               }, impl < T > PartialEq for NodeC < T >
               { fn eq(& self, other : & Self) -> bool { self.data == other.data } },
               impl < T > std :: hash :: Hash for NodeC < T >
               {
                   fn hash < H : std :: hash :: Hasher > (& self, state : & mut H)
                   { self.data.hash(state); }
               }, impl < T > std :: hash :: Hash for NodeB < T >
               {
                   fn hash < H : std :: hash :: Hasher > (& self, state : & mut H)
                   { self.count.hash(state); }
               }, impl < T > Default for NodeA < T >
               {
                   fn default() -> Self
                   {
                       NodeA
                       {
                           value : String :: new(), child_b : None, phantom :
                           PhantomData,
                       }
                   }
               }, impl < T > Default for NodeB < T >
               {
                   fn default() -> Self
                   {
                       NodeB
                       {
                           count : 0, child_a : None, internal : InternalType(0.0),
                           phantom : PhantomData,
                       }
                   }
               }, impl < T > Default for NodeC < T >
               {
                   fn default() -> Self
                   {
                       NodeC
                       { data : 0, ref_a : None, ref_b : None, phantom : PhantomData, }
                   }
               }, impl < T > TestTrait for NodeA < T > where typedef :: generic_types ::
               Container < NodeB < T > , NodeC < T > > : LocalTrait,
               {
                   fn test_method(& self) -> String { format! ("NodeA: {}", self.value) }
               }, impl < T > LocalTrait for NodeB < T >
               {
                   fn local_method(& self) -> usize
                   { self.count + (self.internal.coinduction_method() as usize) }
               }, impl < T : 'static + Clone + Send > CircularTrait for NodeB < T > where
               T : Clone, typedef :: generic_types :: Wrapper < NodeA < T > > :
               LocalTrait, typedef :: generic_types :: MultiGeneric < NodeC < T > , NodeB
               < T > , String > : CircularTrait,
               {
                   fn circular_method(& self) -> Box < dyn CircularTrait >
                   {
                       Box ::
                       new(NodeC :: < T >
                       {
                           data : self.count as i32, ref_a : None, ref_b : None, phantom
                           : PhantomData,
                       })
                   }
               }, impl < T : 'static + Clone + Send > CircularTrait for NodeC < T > where
               T : Clone, NodeA < T > : TestTrait, NodeB < T > : LocalTrait, typedef ::
               generic_types :: Container < NodeA < T > , NodeB < T > > : LocalTrait,
               {
                   fn circular_method(& self) -> Box < dyn CircularTrait >
                   {
                       Box ::
                       new(NodeA :: < T >
                       {
                           value : format! ("Generated from NodeC: {}", self.data),
                           child_b : None, phantom : PhantomData,
                       })
                   }
               }, impl < T : Clone > CoinductionLocalTrait for NodeA < T > where typedef
               :: generic_types :: Wrapper < NodeB < T > > : LocalTrait,
               { fn coinduction_method(& self) -> f64 { self.value.len() as f64 } }, impl
               < T > CoinductionLocalTrait for NodeB < T >
               {
                   fn coinduction_method(& self) -> f64
                   { self.count as f64 * self.internal.0 }
               }, impl < T > CoinductionLocalTrait for NodeC < T > where NodeA < T > :
               TestTrait + CoinductionLocalTrait, NodeB < T > : LocalTrait +
               CoinductionLocalTrait,
               { fn coinduction_method(& self) -> f64 { self.data as f64 * 2.0 } }]
           }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__ConstrainedStruct_temporal_13753066654786479059"
            && e.input == r#""0.2.0", None,
           [typedef :: generic_types :: ConstrainedStruct <
           std :: iter :: Once < NodeC < T > > > : CircularTrait, typedef ::
           generic_types :: Container < NodeB < T > , NodeC < T > > : LocalTrait, typedef
           :: generic_types :: MultiGeneric < NodeC < T > , NodeB < T > , String > :
           CircularTrait, typedef :: generic_types :: Wrapper < NodeB < T > > :
           LocalTrait, typedef :: generic_types :: Container < NodeB < T > , InternalType
           > : LocalTrait, typedef :: generic_types :: Wrapper < NodeA < T > > :
           LocalTrait, typedef :: generic_types :: Container < NodeA < T > , NodeB < T >
           > : LocalTrait], { :: coinduction },
           [LocalTrait, TestTrait, CircularTrait, CoinductionLocalTrait],
           [NodeA, NodeB, NodeC],
           [{
               [],
               [(NodeA < T > : CircularTrait, T : Clone),
               (NodeA < T > : CircularTrait, T : Send),
               (NodeA < T > : CircularTrait, NodeA < T > : Clone),
               (NodeA < T > : CircularTrait, typedef :: generic_types :: Container <
               NodeB < T > , InternalType > : LocalTrait),
               (NodeA < T > : CircularTrait, typedef :: generic_types ::
               ConstrainedStruct < std :: iter :: Once < NodeC < T > > > :
               CircularTrait)], [T : 'static + Clone + Send]
           }, None, None, None, None, None, None, None, None, None, None, None, None,
           None, None, None, None, None, None, None,
           {
               [],
               [(NodeA < T > : TestTrait, typedef :: generic_types :: Container < NodeB <
               T > , NodeC < T > > : LocalTrait)], [T]
           }, { [NodeB < T > : LocalTrait], [], [T] },
           {
               [],
               [(NodeB < T > : CircularTrait, T : Clone),
               (NodeB < T > : CircularTrait, T : Send),
               (NodeB < T > : CircularTrait, T : Clone),
               (NodeB < T > : CircularTrait, typedef :: generic_types :: Wrapper < NodeA
               < T > > : LocalTrait),
               (NodeB < T > : CircularTrait, typedef :: generic_types :: MultiGeneric <
               NodeC < T > , NodeB < T > , String > : CircularTrait)],
               [T : 'static + Clone + Send]
           },
           {
               [],
               [(NodeC < T > : CircularTrait, T : Clone),
               (NodeC < T > : CircularTrait, T : Send),
               (NodeC < T > : CircularTrait, T : Clone),
               (NodeC < T > : CircularTrait, NodeA < T > : TestTrait),
               (NodeC < T > : CircularTrait, NodeB < T > : LocalTrait),
               (NodeC < T > : CircularTrait, typedef :: generic_types :: Container <
               NodeA < T > , NodeB < T > > : LocalTrait),
               (NodeA < T > : TestTrait, typedef :: generic_types :: Container < NodeB <
               T > , NodeC < T > > : LocalTrait)], [T : 'static + Clone + Send]
           },
           {
               [],
               [(NodeA < T > : CoinductionLocalTrait, T : Clone),
               (NodeA < T > : CoinductionLocalTrait, typedef :: generic_types :: Wrapper
               < NodeB < T > > : LocalTrait)], [T : Clone]
           }, { [NodeB < T > : CoinductionLocalTrait], [], [T] },
           {
               [],
               [(NodeC < T > : CoinductionLocalTrait, NodeA < T > : TestTrait),
               (NodeC < T > : CoinductionLocalTrait, NodeA < T > :
               CoinductionLocalTrait),
               (NodeC < T > : CoinductionLocalTrait, NodeB < T > : LocalTrait),
               (NodeC < T > : CoinductionLocalTrait, NodeB < T > :
               CoinductionLocalTrait),
               (NodeA < T > : TestTrait, typedef :: generic_types :: Container < NodeB <
               T > , NodeC < T > > : LocalTrait),
               (NodeA < T > : CoinductionLocalTrait, T : Clone),
               (NodeA < T > : CoinductionLocalTrait, typedef :: generic_types :: Wrapper
               < NodeB < T > > : LocalTrait)], [T]
           }],
           [impl < T : 'static + Clone + Send > CircularTrait for NodeA < T > where NodeA
           < T > : Clone, typedef :: generic_types :: Container < NodeB < T > ,
           InternalType > : LocalTrait, typedef :: generic_types :: ConstrainedStruct <
           std :: iter :: Once < NodeC < T > > > : CircularTrait,
           {
               fn circular_method(& self) -> Box < dyn CircularTrait >
               {
                   Box ::
                   new(NodeB :: < T >
                   {
                       count : self.value.len(), child_a : None, internal :
                       InternalType(42.0), phantom : PhantomData,
                   })
               }
           }, impl < T : Clone > Clone for NodeA < T >
           {
               fn clone(& self) -> Self
               {
                   NodeA
                   {
                       value : self.value.clone(), child_b : self.child_b.clone(),
                       phantom : self.phantom,
                   }
               }
           }, impl < T > std :: fmt :: Debug for NodeA < T >
           {
               fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt ::
               Result
               {
                   f.debug_struct("NodeA").field("value", &
                   self.value).field("child_b", & "<child_b>").finish()
               }
           }, impl < T > PartialEq for NodeA < T >
           { fn eq(& self, other : & Self) -> bool { self.value == other.value } }, impl
           < T > ToString for NodeA < T >
           { fn to_string(& self) -> String { format! ("NodeA({})", self.value) } },
           unsafe impl < T > Send for NodeA < T > {}, unsafe impl < T > Sync for NodeA <
           T > {}, impl < T : Clone > Clone for NodeB < T >
           {
               fn clone(& self) -> Self
               {
                   NodeB
                   {
                       count : self.count, child_a : self.child_a.clone(), internal :
                       self.internal.clone(), phantom : self.phantom,
                   }
               }
           }, impl < T > std :: fmt :: Debug for NodeB < T >
           {
               fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt ::
               Result
               {
                   f.debug_struct("NodeB").field("count", &
                   self.count).field("child_a", &
                   "<child_a>").field("internal", & self.internal).finish()
               }
           }, impl < T > PartialEq for NodeB < T >
           {
               fn eq(& self, other : & Self) -> bool
               { self.count == other.count && self.internal == other.internal }
           }, unsafe impl < T > Send for NodeB < T > {}, unsafe impl < T > Sync for NodeB
           < T > {}, impl < T : Clone > Clone for NodeC < T >
           {
               fn clone(& self) -> Self
               {
                   NodeC
                   {
                       data : self.data, ref_a : self.ref_a.clone(), ref_b :
                       self.ref_b.clone(), phantom : self.phantom,
                   }
               }
           }, impl < T > std :: fmt :: Debug for NodeC < T >
           {
               fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt ::
               Result
               {
                   f.debug_struct("NodeC").field("data", &
                   self.data).field("ref_a", &
                   "<ref_a>").field("ref_b", & "<ref_b>").finish()
               }
           }, impl < T > PartialEq for NodeC < T >
           { fn eq(& self, other : & Self) -> bool { self.data == other.data } }, impl <
           T > std :: hash :: Hash for NodeC < T >
           {
               fn hash < H : std :: hash :: Hasher > (& self, state : & mut H)
               { self.data.hash(state); }
           }, impl < T > std :: hash :: Hash for NodeB < T >
           {
               fn hash < H : std :: hash :: Hasher > (& self, state : & mut H)
               { self.count.hash(state); }
           }, impl < T > Default for NodeA < T >
           {
               fn default() -> Self
               {
                   NodeA
                   { value : String :: new(), child_b : None, phantom : PhantomData, }
               }
           }, impl < T > Default for NodeB < T >
           {
               fn default() -> Self
               {
                   NodeB
                   {
                       count : 0, child_a : None, internal : InternalType(0.0), phantom :
                       PhantomData,
                   }
               }
           }, impl < T > Default for NodeC < T >
           {
               fn default() -> Self
               { NodeC { data : 0, ref_a : None, ref_b : None, phantom : PhantomData, } }
           }, impl < T > TestTrait for NodeA < T > where typedef :: generic_types ::
           Container < NodeB < T > , NodeC < T > > : LocalTrait,
           { fn test_method(& self) -> String { format! ("NodeA: {}", self.value) } },
           impl < T > LocalTrait for NodeB < T >
           {
               fn local_method(& self) -> usize
               { self.count + (self.internal.coinduction_method() as usize) }
           }, impl < T : 'static + Clone + Send > CircularTrait for NodeB < T > where T :
           Clone, typedef :: generic_types :: Wrapper < NodeA < T > > : LocalTrait,
           typedef :: generic_types :: MultiGeneric < NodeC < T > , NodeB < T > , String
           > : CircularTrait,
           {
               fn circular_method(& self) -> Box < dyn CircularTrait >
               {
                   Box ::
                   new(NodeC :: < T >
                   {
                       data : self.count as i32, ref_a : None, ref_b : None, phantom :
                       PhantomData,
                   })
               }
           }, impl < T : 'static + Clone + Send > CircularTrait for NodeC < T > where T :
           Clone, NodeA < T > : TestTrait, NodeB < T > : LocalTrait, typedef ::
           generic_types :: Container < NodeA < T > , NodeB < T > > : LocalTrait,
           {
               fn circular_method(& self) -> Box < dyn CircularTrait >
               {
                   Box ::
                   new(NodeA :: < T >
                   {
                       value : format! ("Generated from NodeC: {}", self.data), child_b :
                       None, phantom : PhantomData,
                   })
               }
           }, impl < T : Clone > CoinductionLocalTrait for NodeA < T > where typedef ::
           generic_types :: Wrapper < NodeB < T > > : LocalTrait,
           { fn coinduction_method(& self) -> f64 { self.value.len() as f64 } }, impl < T
           > CoinductionLocalTrait for NodeB < T >
           {
               fn coinduction_method(& self) -> f64
               { self.count as f64 * self.internal.0 }
           }, impl < T > CoinductionLocalTrait for NodeC < T > where NodeA < T > :
           TestTrait + CoinductionLocalTrait, NodeB < T > : LocalTrait +
           CoinductionLocalTrait,
           { fn coinduction_method(& self) -> f64 { self.data as f64 * 2.0 } }]"#
            && e.to == r#":: coinduction :: __next_step!
           {
               "0.2.0", Typedef
               {
                   predicates :
                   [([__T_12_13753066654786479059], ConstrainedStruct <
                   __T_12_13753066654786479059 > : CircularTrait,
                   [__T_12_13753066654786479059 : Iterator, __T_12_13753066654786479059 :
                   Clone, __T_12_13753066654786479059 : Send, T :: Item : :: std :: fmt
                   :: Debug])]
               },
               [typedef :: generic_types :: ConstrainedStruct <
               std :: iter :: Once < NodeC < T > > > : CircularTrait, typedef ::
               generic_types :: Container < NodeB < T > , NodeC < T > > : LocalTrait,
               typedef :: generic_types :: MultiGeneric < NodeC < T > , NodeB < T > ,
               String > : CircularTrait, typedef :: generic_types :: Wrapper < NodeB < T
               > > : LocalTrait, typedef :: generic_types :: Container < NodeB < T > ,
               InternalType > : LocalTrait, typedef :: generic_types :: Wrapper < NodeA <
               T > > : LocalTrait, typedef :: generic_types :: Container < NodeA < T > ,
               NodeB < T > > : LocalTrait], { :: coinduction },
               [LocalTrait, TestTrait, CircularTrait, CoinductionLocalTrait],
               [NodeA, NodeB, NodeC],
               [{
                   [],
                   [(NodeA < T > : CircularTrait, T : Clone),
                   (NodeA < T > : CircularTrait, T : Send),
                   (NodeA < T > : CircularTrait, NodeA < T > : Clone),
                   (NodeA < T > : CircularTrait, typedef :: generic_types :: Container <
                   NodeB < T > , InternalType > : LocalTrait),
                   (NodeA < T > : CircularTrait, typedef :: generic_types ::
                   ConstrainedStruct < std :: iter :: Once < NodeC < T > > > :
                   CircularTrait)], [T : 'static + Clone + Send]
               }, None, None, None, None, None, None, None, None, None, None, None, None,
               None, None, None, None, None, None, None,
               {
                   [],
                   [(NodeA < T > : TestTrait, typedef :: generic_types :: Container <
                   NodeB < T > , NodeC < T > > : LocalTrait)], [T]
               }, { [NodeB < T > : LocalTrait], [], [T] },
               {
                   [],
                   [(NodeB < T > : CircularTrait, T : Clone),
                   (NodeB < T > : CircularTrait, T : Send),
                   (NodeB < T > : CircularTrait, T : Clone),
                   (NodeB < T > : CircularTrait, typedef :: generic_types :: Wrapper <
                   NodeA < T > > : LocalTrait),
                   (NodeB < T > : CircularTrait, typedef :: generic_types :: MultiGeneric
                   < NodeC < T > , NodeB < T > , String > : CircularTrait)],
                   [T : 'static + Clone + Send]
               },
               {
                   [],
                   [(NodeC < T > : CircularTrait, T : Clone),
                   (NodeC < T > : CircularTrait, T : Send),
                   (NodeC < T > : CircularTrait, T : Clone),
                   (NodeC < T > : CircularTrait, NodeA < T > : TestTrait),
                   (NodeC < T > : CircularTrait, NodeB < T > : LocalTrait),
                   (NodeC < T > : CircularTrait, typedef :: generic_types :: Container <
                   NodeA < T > , NodeB < T > > : LocalTrait),
                   (NodeA < T > : TestTrait, typedef :: generic_types :: Container <
                   NodeB < T > , NodeC < T > > : LocalTrait)],
                   [T : 'static + Clone + Send]
               },
               {
                   [],
                   [(NodeA < T > : CoinductionLocalTrait, T : Clone),
                   (NodeA < T > : CoinductionLocalTrait, typedef :: generic_types ::
                   Wrapper < NodeB < T > > : LocalTrait)], [T : Clone]
               }, { [NodeB < T > : CoinductionLocalTrait], [], [T] },
               {
                   [],
                   [(NodeC < T > : CoinductionLocalTrait, NodeA < T > : TestTrait),
                   (NodeC < T > : CoinductionLocalTrait, NodeA < T > :
                   CoinductionLocalTrait),
                   (NodeC < T > : CoinductionLocalTrait, NodeB < T > : LocalTrait),
                   (NodeC < T > : CoinductionLocalTrait, NodeB < T > :
                   CoinductionLocalTrait),
                   (NodeA < T > : TestTrait, typedef :: generic_types :: Container <
                   NodeB < T > , NodeC < T > > : LocalTrait),
                   (NodeA < T > : CoinductionLocalTrait, T : Clone),
                   (NodeA < T > : CoinductionLocalTrait, typedef :: generic_types ::
                   Wrapper < NodeB < T > > : LocalTrait)], [T]
               }],
               [impl < T : 'static + Clone + Send > CircularTrait for NodeA < T > where
               NodeA < T > : Clone, typedef :: generic_types :: Container < NodeB < T > ,
               InternalType > : LocalTrait, typedef :: generic_types :: ConstrainedStruct
               < std :: iter :: Once < NodeC < T > > > : CircularTrait,
               {
                   fn circular_method(& self) -> Box < dyn CircularTrait >
                   {
                       Box ::
                       new(NodeB :: < T >
                       {
                           count : self.value.len(), child_a : None, internal :
                           InternalType(42.0), phantom : PhantomData,
                       })
                   }
               }, impl < T : Clone > Clone for NodeA < T >
               {
                   fn clone(& self) -> Self
                   {
                       NodeA
                       {
                           value : self.value.clone(), child_b : self.child_b.clone(),
                           phantom : self.phantom,
                       }
                   }
               }, impl < T > std :: fmt :: Debug for NodeA < T >
               {
                   fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt
                   :: Result
                   {
                       f.debug_struct("NodeA").field("value", &
                       self.value).field("child_b", & "<child_b>").finish()
                   }
               }, impl < T > PartialEq for NodeA < T >
               { fn eq(& self, other : & Self) -> bool { self.value == other.value } },
               impl < T > ToString for NodeA < T >
               { fn to_string(& self) -> String { format! ("NodeA({})", self.value) } },
               unsafe impl < T > Send for NodeA < T > {}, unsafe impl < T > Sync for
               NodeA < T > {}, impl < T : Clone > Clone for NodeB < T >
               {
                   fn clone(& self) -> Self
                   {
                       NodeB
                       {
                           count : self.count, child_a : self.child_a.clone(), internal :
                           self.internal.clone(), phantom : self.phantom,
                       }
                   }
               }, impl < T > std :: fmt :: Debug for NodeB < T >
               {
                   fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt
                   :: Result
                   {
                       f.debug_struct("NodeB").field("count", &
                       self.count).field("child_a", &
                       "<child_a>").field("internal", & self.internal).finish()
                   }
               }, impl < T > PartialEq for NodeB < T >
               {
                   fn eq(& self, other : & Self) -> bool
                   { self.count == other.count && self.internal == other.internal }
               }, unsafe impl < T > Send for NodeB < T > {}, unsafe impl < T > Sync for
               NodeB < T > {}, impl < T : Clone > Clone for NodeC < T >
               {
                   fn clone(& self) -> Self
                   {
                       NodeC
                       {
                           data : self.data, ref_a : self.ref_a.clone(), ref_b :
                           self.ref_b.clone(), phantom : self.phantom,
                       }
                   }
               }, impl < T > std :: fmt :: Debug for NodeC < T >
               {
                   fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt
                   :: Result
                   {
                       f.debug_struct("NodeC").field("data", &
                       self.data).field("ref_a", &
                       "<ref_a>").field("ref_b", & "<ref_b>").finish()
                   }
               }, impl < T > PartialEq for NodeC < T >
               { fn eq(& self, other : & Self) -> bool { self.data == other.data } },
               impl < T > std :: hash :: Hash for NodeC < T >
               {
                   fn hash < H : std :: hash :: Hasher > (& self, state : & mut H)
                   { self.data.hash(state); }
               }, impl < T > std :: hash :: Hash for NodeB < T >
               {
                   fn hash < H : std :: hash :: Hasher > (& self, state : & mut H)
                   { self.count.hash(state); }
               }, impl < T > Default for NodeA < T >
               {
                   fn default() -> Self
                   {
                       NodeA
                       {
                           value : String :: new(), child_b : None, phantom :
                           PhantomData,
                       }
                   }
               }, impl < T > Default for NodeB < T >
               {
                   fn default() -> Self
                   {
                       NodeB
                       {
                           count : 0, child_a : None, internal : InternalType(0.0),
                           phantom : PhantomData,
                       }
                   }
               }, impl < T > Default for NodeC < T >
               {
                   fn default() -> Self
                   {
                       NodeC
                       { data : 0, ref_a : None, ref_b : None, phantom : PhantomData, }
                   }
               }, impl < T > TestTrait for NodeA < T > where typedef :: generic_types ::
               Container < NodeB < T > , NodeC < T > > : LocalTrait,
               {
                   fn test_method(& self) -> String { format! ("NodeA: {}", self.value) }
               }, impl < T > LocalTrait for NodeB < T >
               {
                   fn local_method(& self) -> usize
                   { self.count + (self.internal.coinduction_method() as usize) }
               }, impl < T : 'static + Clone + Send > CircularTrait for NodeB < T > where
               T : Clone, typedef :: generic_types :: Wrapper < NodeA < T > > :
               LocalTrait, typedef :: generic_types :: MultiGeneric < NodeC < T > , NodeB
               < T > , String > : CircularTrait,
               {
                   fn circular_method(& self) -> Box < dyn CircularTrait >
                   {
                       Box ::
                       new(NodeC :: < T >
                       {
                           data : self.count as i32, ref_a : None, ref_b : None, phantom
                           : PhantomData,
                       })
                   }
               }, impl < T : 'static + Clone + Send > CircularTrait for NodeC < T > where
               T : Clone, NodeA < T > : TestTrait, NodeB < T > : LocalTrait, typedef ::
               generic_types :: Container < NodeA < T > , NodeB < T > > : LocalTrait,
               {
                   fn circular_method(& self) -> Box < dyn CircularTrait >
                   {
                       Box ::
                       new(NodeA :: < T >
                       {
                           value : format! ("Generated from NodeC: {}", self.data),
                           child_b : None, phantom : PhantomData,
                       })
                   }
               }, impl < T : Clone > CoinductionLocalTrait for NodeA < T > where typedef
               :: generic_types :: Wrapper < NodeB < T > > : LocalTrait,
               { fn coinduction_method(& self) -> f64 { self.value.len() as f64 } }, impl
               < T > CoinductionLocalTrait for NodeB < T >
               {
                   fn coinduction_method(& self) -> f64
                   { self.count as f64 * self.internal.0 }
               }, impl < T > CoinductionLocalTrait for NodeC < T > where NodeA < T > :
               TestTrait + CoinductionLocalTrait, NodeB < T > : LocalTrait +
               CoinductionLocalTrait,
               { fn coinduction_method(& self) -> f64 { self.data as f64 * 2.0 } }]
           }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__Container_temporal_13753066654786479059"
            && e.input == r#""0.2.0", None,
           [typedef :: generic_types :: Container < NodeB < T >, NodeC < T > > :
           LocalTrait, typedef :: generic_types :: MultiGeneric < NodeC < T > , NodeB < T
           > , String > : CircularTrait, typedef :: generic_types :: Wrapper < NodeB < T
           > > : LocalTrait, typedef :: generic_types :: Container < NodeB < T > ,
           InternalType > : LocalTrait, typedef :: generic_types :: Wrapper < NodeA < T >
           > : LocalTrait, typedef :: generic_types :: Container < NodeA < T > , NodeB <
           T > > : LocalTrait], { :: coinduction },
           [LocalTrait, TestTrait, CircularTrait, CoinductionLocalTrait],
           [NodeC, NodeA, NodeB],
           [{
               [],
               [(NodeA < T > : CircularTrait, T : Clone),
               (NodeA < T > : CircularTrait, T : Send),
               (NodeA < T > : CircularTrait, NodeA < T > : Clone),
               (NodeA < T > : CircularTrait, typedef :: generic_types :: Container <
               NodeB < T > , InternalType > : LocalTrait),
               (NodeA < T > : CircularTrait, typedef :: generic_types ::
               ConstrainedStruct < std :: iter :: Once < NodeC < T > > > :
               CircularTrait)], [T : 'static + Clone + Send]
           }, None, None, None, None, None, None, None, None, None, None, None, None,
           None, None, None, None, None, None, None,
           {
               [],
               [(NodeA < T > : TestTrait, typedef :: generic_types :: Container < NodeB <
               T > , NodeC < T > > : LocalTrait)], [T]
           }, { [NodeB < T > : LocalTrait], [], [T] },
           {
               [],
               [(NodeB < T > : CircularTrait, T : Clone),
               (NodeB < T > : CircularTrait, T : Send),
               (NodeB < T > : CircularTrait, T : Clone),
               (NodeB < T > : CircularTrait, typedef :: generic_types :: Wrapper < NodeA
               < T > > : LocalTrait),
               (NodeB < T > : CircularTrait, typedef :: generic_types :: MultiGeneric <
               NodeC < T > , NodeB < T > , String > : CircularTrait)],
               [T : 'static + Clone + Send]
           },
           {
               [],
               [(NodeC < T > : CircularTrait, T : Clone),
               (NodeC < T > : CircularTrait, T : Send),
               (NodeC < T > : CircularTrait, T : Clone),
               (NodeC < T > : CircularTrait, NodeA < T > : TestTrait),
               (NodeC < T > : CircularTrait, NodeB < T > : LocalTrait),
               (NodeC < T > : CircularTrait, typedef :: generic_types :: Container <
               NodeA < T > , NodeB < T > > : LocalTrait),
               (NodeA < T > : TestTrait, typedef :: generic_types :: Container < NodeB <
               T > , NodeC < T > > : LocalTrait)], [T : 'static + Clone + Send]
           },
           {
               [],
               [(NodeA < T > : CoinductionLocalTrait, T : Clone),
               (NodeA < T > : CoinductionLocalTrait, typedef :: generic_types :: Wrapper
               < NodeB < T > > : LocalTrait)], [T : Clone]
           }, { [NodeB < T > : CoinductionLocalTrait], [], [T] },
           {
               [],
               [(NodeC < T > : CoinductionLocalTrait, NodeA < T > : TestTrait),
               (NodeC < T > : CoinductionLocalTrait, NodeA < T > :
               CoinductionLocalTrait),
               (NodeC < T > : CoinductionLocalTrait, NodeB < T > : LocalTrait),
               (NodeC < T > : CoinductionLocalTrait, NodeB < T > :
               CoinductionLocalTrait),
               (NodeA < T > : TestTrait, typedef :: generic_types :: Container < NodeB <
               T > , NodeC < T > > : LocalTrait),
               (NodeA < T > : CoinductionLocalTrait, T : Clone),
               (NodeA < T > : CoinductionLocalTrait, typedef :: generic_types :: Wrapper
               < NodeB < T > > : LocalTrait)], [T]
           }],
           [impl < T : 'static + Clone + Send > CircularTrait for NodeA < T > where NodeA
           < T > : Clone, typedef :: generic_types :: Container < NodeB < T > ,
           InternalType > : LocalTrait, typedef :: generic_types :: ConstrainedStruct <
           std :: iter :: Once < NodeC < T > > > : CircularTrait,
           {
               fn circular_method(& self) -> Box < dyn CircularTrait >
               {
                   Box ::
                   new(NodeB :: < T >
                   {
                       count : self.value.len(), child_a : None, internal :
                       InternalType(42.0), phantom : PhantomData,
                   })
               }
           }, impl < T : Clone > Clone for NodeA < T >
           {
               fn clone(& self) -> Self
               {
                   NodeA
                   {
                       value : self.value.clone(), child_b : self.child_b.clone(),
                       phantom : self.phantom,
                   }
               }
           }, impl < T > std :: fmt :: Debug for NodeA < T >
           {
               fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt ::
               Result
               {
                   f.debug_struct("NodeA").field("value", &
                   self.value).field("child_b", & "<child_b>").finish()
               }
           }, impl < T > PartialEq for NodeA < T >
           { fn eq(& self, other : & Self) -> bool { self.value == other.value } }, impl
           < T > ToString for NodeA < T >
           { fn to_string(& self) -> String { format! ("NodeA({})", self.value) } },
           unsafe impl < T > Send for NodeA < T > {}, unsafe impl < T > Sync for NodeA <
           T > {}, impl < T : Clone > Clone for NodeB < T >
           {
               fn clone(& self) -> Self
               {
                   NodeB
                   {
                       count : self.count, child_a : self.child_a.clone(), internal :
                       self.internal.clone(), phantom : self.phantom,
                   }
               }
           }, impl < T > std :: fmt :: Debug for NodeB < T >
           {
               fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt ::
               Result
               {
                   f.debug_struct("NodeB").field("count", &
                   self.count).field("child_a", &
                   "<child_a>").field("internal", & self.internal).finish()
               }
           }, impl < T > PartialEq for NodeB < T >
           {
               fn eq(& self, other : & Self) -> bool
               { self.count == other.count && self.internal == other.internal }
           }, unsafe impl < T > Send for NodeB < T > {}, unsafe impl < T > Sync for NodeB
           < T > {}, impl < T : Clone > Clone for NodeC < T >
           {
               fn clone(& self) -> Self
               {
                   NodeC
                   {
                       data : self.data, ref_a : self.ref_a.clone(), ref_b :
                       self.ref_b.clone(), phantom : self.phantom,
                   }
               }
           }, impl < T > std :: fmt :: Debug for NodeC < T >
           {
               fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt ::
               Result
               {
                   f.debug_struct("NodeC").field("data", &
                   self.data).field("ref_a", &
                   "<ref_a>").field("ref_b", & "<ref_b>").finish()
               }
           }, impl < T > PartialEq for NodeC < T >
           { fn eq(& self, other : & Self) -> bool { self.data == other.data } }, impl <
           T > std :: hash :: Hash for NodeC < T >
           {
               fn hash < H : std :: hash :: Hasher > (& self, state : & mut H)
               { self.data.hash(state); }
           }, impl < T > std :: hash :: Hash for NodeB < T >
           {
               fn hash < H : std :: hash :: Hasher > (& self, state : & mut H)
               { self.count.hash(state); }
           }, impl < T > Default for NodeA < T >
           {
               fn default() -> Self
               {
                   NodeA
                   { value : String :: new(), child_b : None, phantom : PhantomData, }
               }
           }, impl < T > Default for NodeB < T >
           {
               fn default() -> Self
               {
                   NodeB
                   {
                       count : 0, child_a : None, internal : InternalType(0.0), phantom :
                       PhantomData,
                   }
               }
           }, impl < T > Default for NodeC < T >
           {
               fn default() -> Self
               { NodeC { data : 0, ref_a : None, ref_b : None, phantom : PhantomData, } }
           }, impl < T > TestTrait for NodeA < T > where typedef :: generic_types ::
           Container < NodeB < T > , NodeC < T > > : LocalTrait,
           { fn test_method(& self) -> String { format! ("NodeA: {}", self.value) } },
           impl < T > LocalTrait for NodeB < T >
           {
               fn local_method(& self) -> usize
               { self.count + (self.internal.coinduction_method() as usize) }
           }, impl < T : 'static + Clone + Send > CircularTrait for NodeB < T > where T :
           Clone, typedef :: generic_types :: Wrapper < NodeA < T > > : LocalTrait,
           typedef :: generic_types :: MultiGeneric < NodeC < T > , NodeB < T > , String
           > : CircularTrait,
           {
               fn circular_method(& self) -> Box < dyn CircularTrait >
               {
                   Box ::
                   new(NodeC :: < T >
                   {
                       data : self.count as i32, ref_a : None, ref_b : None, phantom :
                       PhantomData,
                   })
               }
           }, impl < T : 'static + Clone + Send > CircularTrait for NodeC < T > where T :
           Clone, NodeA < T > : TestTrait, NodeB < T > : LocalTrait, typedef ::
           generic_types :: Container < NodeA < T > , NodeB < T > > : LocalTrait,
           {
               fn circular_method(& self) -> Box < dyn CircularTrait >
               {
                   Box ::
                   new(NodeA :: < T >
                   {
                       value : format! ("Generated from NodeC: {}", self.data), child_b :
                       None, phantom : PhantomData,
                   })
               }
           }, impl < T : Clone > CoinductionLocalTrait for NodeA < T > where typedef ::
           generic_types :: Wrapper < NodeB < T > > : LocalTrait,
           { fn coinduction_method(& self) -> f64 { self.value.len() as f64 } }, impl < T
           > CoinductionLocalTrait for NodeB < T >
           {
               fn coinduction_method(& self) -> f64
               { self.count as f64 * self.internal.0 }
           }, impl < T > CoinductionLocalTrait for NodeC < T > where NodeA < T > :
           TestTrait + CoinductionLocalTrait, NodeB < T > : LocalTrait +
           CoinductionLocalTrait,
           { fn coinduction_method(& self) -> f64 { self.data as f64 * 2.0 } }]"#
            && e.to == r#":: coinduction :: __next_step!
           {
               "0.2.0", Typedef
               {
                   predicates :
                   [([__T_7_13753066654786479059, __U_7_13753066654786479059], Container
                   < __T_7_13753066654786479059, __U_7_13753066654786479059 > :
                   TestTrait,
                   [__T_7_13753066654786479059 : Clone, __T_7_13753066654786479059 : ::
                   std :: fmt :: Debug, __T_7_13753066654786479059 : Send,
                   __U_7_13753066654786479059 : :: std :: fmt :: Debug,
                   __U_7_13753066654786479059 : Default, __U_7_13753066654786479059 :
                   Sync]),
                   ([__T_9_13753066654786479059, __U_9_13753066654786479059], Container <
                   __T_9_13753066654786479059, __U_9_13753066654786479059 > : LocalTrait,
                   [__T_9_13753066654786479059 : Clone, __T_9_13753066654786479059 :
                   Send, __T_9_13753066654786479059 : Sync, __U_9_13753066654786479059 :
                   :: std :: fmt :: Debug, __U_9_13753066654786479059 : Hash]),
                   ([__T_13_13753066654786479059, __U_13_13753066654786479059], Container
                   < __T_13_13753066654786479059, __U_13_13753066654786479059 > :
                   ExtendedTrait,
                   [__T_13_13753066654786479059 : PartialEq, __T_13_13753066654786479059
                   : Clone, __U_13_13753066654786479059 : Default,
                   __U_13_13753066654786479059 : Send])]
               },
               [typedef :: generic_types :: Container < NodeB < T >, NodeC < T > > :
               LocalTrait, typedef :: generic_types :: MultiGeneric < NodeC < T > , NodeB
               < T > , String > : CircularTrait, typedef :: generic_types :: Wrapper <
               NodeB < T > > : LocalTrait, typedef :: generic_types :: Container < NodeB
               < T > , InternalType > : LocalTrait, typedef :: generic_types :: Wrapper <
               NodeA < T > > : LocalTrait, typedef :: generic_types :: Container < NodeA
               < T > , NodeB < T > > : LocalTrait], { :: coinduction },
               [LocalTrait, TestTrait, CircularTrait, CoinductionLocalTrait],
               [NodeC, NodeA, NodeB],
               [{
                   [],
                   [(NodeA < T > : CircularTrait, T : Clone),
                   (NodeA < T > : CircularTrait, T : Send),
                   (NodeA < T > : CircularTrait, NodeA < T > : Clone),
                   (NodeA < T > : CircularTrait, typedef :: generic_types :: Container <
                   NodeB < T > , InternalType > : LocalTrait),
                   (NodeA < T > : CircularTrait, typedef :: generic_types ::
                   ConstrainedStruct < std :: iter :: Once < NodeC < T > > > :
                   CircularTrait)], [T : 'static + Clone + Send]
               }, None, None, None, None, None, None, None, None, None, None, None, None,
               None, None, None, None, None, None, None,
               {
                   [],
                   [(NodeA < T > : TestTrait, typedef :: generic_types :: Container <
                   NodeB < T > , NodeC < T > > : LocalTrait)], [T]
               }, { [NodeB < T > : LocalTrait], [], [T] },
               {
                   [],
                   [(NodeB < T > : CircularTrait, T : Clone),
                   (NodeB < T > : CircularTrait, T : Send),
                   (NodeB < T > : CircularTrait, T : Clone),
                   (NodeB < T > : CircularTrait, typedef :: generic_types :: Wrapper <
                   NodeA < T > > : LocalTrait),
                   (NodeB < T > : CircularTrait, typedef :: generic_types :: MultiGeneric
                   < NodeC < T > , NodeB < T > , String > : CircularTrait)],
                   [T : 'static + Clone + Send]
               },
               {
                   [],
                   [(NodeC < T > : CircularTrait, T : Clone),
                   (NodeC < T > : CircularTrait, T : Send),
                   (NodeC < T > : CircularTrait, T : Clone),
                   (NodeC < T > : CircularTrait, NodeA < T > : TestTrait),
                   (NodeC < T > : CircularTrait, NodeB < T > : LocalTrait),
                   (NodeC < T > : CircularTrait, typedef :: generic_types :: Container <
                   NodeA < T > , NodeB < T > > : LocalTrait),
                   (NodeA < T > : TestTrait, typedef :: generic_types :: Container <
                   NodeB < T > , NodeC < T > > : LocalTrait)],
                   [T : 'static + Clone + Send]
               },
               {
                   [],
                   [(NodeA < T > : CoinductionLocalTrait, T : Clone),
                   (NodeA < T > : CoinductionLocalTrait, typedef :: generic_types ::
                   Wrapper < NodeB < T > > : LocalTrait)], [T : Clone]
               }, { [NodeB < T > : CoinductionLocalTrait], [], [T] },
               {
                   [],
                   [(NodeC < T > : CoinductionLocalTrait, NodeA < T > : TestTrait),
                   (NodeC < T > : CoinductionLocalTrait, NodeA < T > :
                   CoinductionLocalTrait),
                   (NodeC < T > : CoinductionLocalTrait, NodeB < T > : LocalTrait),
                   (NodeC < T > : CoinductionLocalTrait, NodeB < T > :
                   CoinductionLocalTrait),
                   (NodeA < T > : TestTrait, typedef :: generic_types :: Container <
                   NodeB < T > , NodeC < T > > : LocalTrait),
                   (NodeA < T > : CoinductionLocalTrait, T : Clone),
                   (NodeA < T > : CoinductionLocalTrait, typedef :: generic_types ::
                   Wrapper < NodeB < T > > : LocalTrait)], [T]
               }],
               [impl < T : 'static + Clone + Send > CircularTrait for NodeA < T > where
               NodeA < T > : Clone, typedef :: generic_types :: Container < NodeB < T > ,
               InternalType > : LocalTrait, typedef :: generic_types :: ConstrainedStruct
               < std :: iter :: Once < NodeC < T > > > : CircularTrait,
               {
                   fn circular_method(& self) -> Box < dyn CircularTrait >
                   {
                       Box ::
                       new(NodeB :: < T >
                       {
                           count : self.value.len(), child_a : None, internal :
                           InternalType(42.0), phantom : PhantomData,
                       })
                   }
               }, impl < T : Clone > Clone for NodeA < T >
               {
                   fn clone(& self) -> Self
                   {
                       NodeA
                       {
                           value : self.value.clone(), child_b : self.child_b.clone(),
                           phantom : self.phantom,
                       }
                   }
               }, impl < T > std :: fmt :: Debug for NodeA < T >
               {
                   fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt
                   :: Result
                   {
                       f.debug_struct("NodeA").field("value", &
                       self.value).field("child_b", & "<child_b>").finish()
                   }
               }, impl < T > PartialEq for NodeA < T >
               { fn eq(& self, other : & Self) -> bool { self.value == other.value } },
               impl < T > ToString for NodeA < T >
               { fn to_string(& self) -> String { format! ("NodeA({})", self.value) } },
               unsafe impl < T > Send for NodeA < T > {}, unsafe impl < T > Sync for
               NodeA < T > {}, impl < T : Clone > Clone for NodeB < T >
               {
                   fn clone(& self) -> Self
                   {
                       NodeB
                       {
                           count : self.count, child_a : self.child_a.clone(), internal :
                           self.internal.clone(), phantom : self.phantom,
                       }
                   }
               }, impl < T > std :: fmt :: Debug for NodeB < T >
               {
                   fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt
                   :: Result
                   {
                       f.debug_struct("NodeB").field("count", &
                       self.count).field("child_a", &
                       "<child_a>").field("internal", & self.internal).finish()
                   }
               }, impl < T > PartialEq for NodeB < T >
               {
                   fn eq(& self, other : & Self) -> bool
                   { self.count == other.count && self.internal == other.internal }
               }, unsafe impl < T > Send for NodeB < T > {}, unsafe impl < T > Sync for
               NodeB < T > {}, impl < T : Clone > Clone for NodeC < T >
               {
                   fn clone(& self) -> Self
                   {
                       NodeC
                       {
                           data : self.data, ref_a : self.ref_a.clone(), ref_b :
                           self.ref_b.clone(), phantom : self.phantom,
                       }
                   }
               }, impl < T > std :: fmt :: Debug for NodeC < T >
               {
                   fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt
                   :: Result
                   {
                       f.debug_struct("NodeC").field("data", &
                       self.data).field("ref_a", &
                       "<ref_a>").field("ref_b", & "<ref_b>").finish()
                   }
               }, impl < T > PartialEq for NodeC < T >
               { fn eq(& self, other : & Self) -> bool { self.data == other.data } },
               impl < T > std :: hash :: Hash for NodeC < T >
               {
                   fn hash < H : std :: hash :: Hasher > (& self, state : & mut H)
                   { self.data.hash(state); }
               }, impl < T > std :: hash :: Hash for NodeB < T >
               {
                   fn hash < H : std :: hash :: Hasher > (& self, state : & mut H)
                   { self.count.hash(state); }
               }, impl < T > Default for NodeA < T >
               {
                   fn default() -> Self
                   {
                       NodeA
                       {
                           value : String :: new(), child_b : None, phantom :
                           PhantomData,
                       }
                   }
               }, impl < T > Default for NodeB < T >
               {
                   fn default() -> Self
                   {
                       NodeB
                       {
                           count : 0, child_a : None, internal : InternalType(0.0),
                           phantom : PhantomData,
                       }
                   }
               }, impl < T > Default for NodeC < T >
               {
                   fn default() -> Self
                   {
                       NodeC
                       { data : 0, ref_a : None, ref_b : None, phantom : PhantomData, }
                   }
               }, impl < T > TestTrait for NodeA < T > where typedef :: generic_types ::
               Container < NodeB < T > , NodeC < T > > : LocalTrait,
               {
                   fn test_method(& self) -> String { format! ("NodeA: {}", self.value) }
               }, impl < T > LocalTrait for NodeB < T >
               {
                   fn local_method(& self) -> usize
                   { self.count + (self.internal.coinduction_method() as usize) }
               }, impl < T : 'static + Clone + Send > CircularTrait for NodeB < T > where
               T : Clone, typedef :: generic_types :: Wrapper < NodeA < T > > :
               LocalTrait, typedef :: generic_types :: MultiGeneric < NodeC < T > , NodeB
               < T > , String > : CircularTrait,
               {
                   fn circular_method(& self) -> Box < dyn CircularTrait >
                   {
                       Box ::
                       new(NodeC :: < T >
                       {
                           data : self.count as i32, ref_a : None, ref_b : None, phantom
                           : PhantomData,
                       })
                   }
               }, impl < T : 'static + Clone + Send > CircularTrait for NodeC < T > where
               T : Clone, NodeA < T > : TestTrait, NodeB < T > : LocalTrait, typedef ::
               generic_types :: Container < NodeA < T > , NodeB < T > > : LocalTrait,
               {
                   fn circular_method(& self) -> Box < dyn CircularTrait >
                   {
                       Box ::
                       new(NodeA :: < T >
                       {
                           value : format! ("Generated from NodeC: {}", self.data),
                           child_b : None, phantom : PhantomData,
                       })
                   }
               }, impl < T : Clone > CoinductionLocalTrait for NodeA < T > where typedef
               :: generic_types :: Wrapper < NodeB < T > > : LocalTrait,
               { fn coinduction_method(& self) -> f64 { self.value.len() as f64 } }, impl
               < T > CoinductionLocalTrait for NodeB < T >
               {
                   fn coinduction_method(& self) -> f64
                   { self.count as f64 * self.internal.0 }
               }, impl < T > CoinductionLocalTrait for NodeC < T > where NodeA < T > :
               TestTrait + CoinductionLocalTrait, NodeB < T > : LocalTrait +
               CoinductionLocalTrait,
               { fn coinduction_method(& self) -> f64 { self.data as f64 * 2.0 } }]
           }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__LocalTrait_temporal_3044150873991545574"
            && e.input == r#""0.2.0", None,
           [typedef :: generic_types :: Container < NodeB < T > , NodeC < T > > :
           LocalTrait, typedef :: generic_types :: MultiGeneric < NodeC < T > , NodeB < T
           > , String > : CircularTrait, typedef :: generic_types :: Wrapper < NodeB < T
           > > : LocalTrait, typedef :: generic_types :: Container < NodeB < T > ,
           InternalType > : LocalTrait, typedef :: generic_types :: Wrapper < NodeA < T >
           > : LocalTrait, typedef :: generic_types :: Container < NodeA < T > , NodeB <
           T > > : LocalTrait], { :: coinduction },
           [LocalTrait, TestTrait, CircularTrait, CoinductionLocalTrait],
           [NodeC, NodeA, NodeB],
           [{
               [],
               [(NodeA < T > : CircularTrait, T : Clone),
               (NodeA < T > : CircularTrait, T : Send),
               (NodeA < T > : CircularTrait, NodeA < T > : Clone),
               (NodeA < T > : CircularTrait, typedef :: generic_types :: Container <
               NodeB < T > , InternalType > : LocalTrait),
               (NodeA < T > : CircularTrait, typedef :: generic_types ::
               ConstrainedStruct < std :: iter :: Once < NodeC < T > > > :
               CircularTrait)], [T : 'static + Clone + Send]
           }, None, None, None, None, None, None, None, None, None, None, None, None,
           None, None, None, None, None, None, None,
           {
               [],
               [(NodeA < T > : TestTrait, typedef :: generic_types :: Container < NodeB <
               T > , NodeC < T > > : LocalTrait)], [T]
           }, { [NodeB < T > : LocalTrait], [], [T] },
           {
               [],
               [(NodeB < T > : CircularTrait, T : Clone),
               (NodeB < T > : CircularTrait, T : Send),
               (NodeB < T > : CircularTrait, T : Clone),
               (NodeB < T > : CircularTrait, typedef :: generic_types :: Wrapper < NodeA
               < T > > : LocalTrait),
               (NodeB < T > : CircularTrait, typedef :: generic_types :: MultiGeneric <
               NodeC < T > , NodeB < T > , String > : CircularTrait)],
               [T : 'static + Clone + Send]
           },
           {
               [],
               [(NodeC < T > : CircularTrait, T : Clone),
               (NodeC < T > : CircularTrait, T : Send),
               (NodeC < T > : CircularTrait, T : Clone),
               (NodeC < T > : CircularTrait, NodeA < T > : TestTrait),
               (NodeC < T > : CircularTrait, NodeB < T > : LocalTrait),
               (NodeC < T > : CircularTrait, typedef :: generic_types :: Container <
               NodeA < T > , NodeB < T > > : LocalTrait),
               (NodeA < T > : TestTrait, typedef :: generic_types :: Container < NodeB <
               T > , NodeC < T > > : LocalTrait)], [T : 'static + Clone + Send]
           },
           {
               [],
               [(NodeA < T > : CoinductionLocalTrait, T : Clone),
               (NodeA < T > : CoinductionLocalTrait, typedef :: generic_types :: Wrapper
               < NodeB < T > > : LocalTrait)], [T : Clone]
           }, { [NodeB < T > : CoinductionLocalTrait], [], [T] },
           {
               [],
               [(NodeC < T > : CoinductionLocalTrait, NodeA < T > : TestTrait),
               (NodeC < T > : CoinductionLocalTrait, NodeA < T > :
               CoinductionLocalTrait),
               (NodeC < T > : CoinductionLocalTrait, NodeB < T > : LocalTrait),
               (NodeC < T > : CoinductionLocalTrait, NodeB < T > :
               CoinductionLocalTrait),
               (NodeA < T > : TestTrait, typedef :: generic_types :: Container < NodeB <
               T > , NodeC < T > > : LocalTrait),
               (NodeA < T > : CoinductionLocalTrait, T : Clone),
               (NodeA < T > : CoinductionLocalTrait, typedef :: generic_types :: Wrapper
               < NodeB < T > > : LocalTrait)], [T]
           }],
           [impl < T : 'static + Clone + Send > CircularTrait for NodeA < T > where NodeA
           < T > : Clone, typedef :: generic_types :: Container < NodeB < T > ,
           InternalType > : LocalTrait, typedef :: generic_types :: ConstrainedStruct <
           std :: iter :: Once < NodeC < T > > > : CircularTrait,
           {
               fn circular_method(& self) -> Box < dyn CircularTrait >
               {
                   Box ::
                   new(NodeB :: < T >
                   {
                       count : self.value.len(), child_a : None, internal :
                       InternalType(42.0), phantom : PhantomData,
                   })
               }
           }, impl < T : Clone > Clone for NodeA < T >
           {
               fn clone(& self) -> Self
               {
                   NodeA
                   {
                       value : self.value.clone(), child_b : self.child_b.clone(),
                       phantom : self.phantom,
                   }
               }
           }, impl < T > std :: fmt :: Debug for NodeA < T >
           {
               fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt ::
               Result
               {
                   f.debug_struct("NodeA").field("value", &
                   self.value).field("child_b", & "<child_b>").finish()
               }
           }, impl < T > PartialEq for NodeA < T >
           { fn eq(& self, other : & Self) -> bool { self.value == other.value } }, impl
           < T > ToString for NodeA < T >
           { fn to_string(& self) -> String { format! ("NodeA({})", self.value) } },
           unsafe impl < T > Send for NodeA < T > {}, unsafe impl < T > Sync for NodeA <
           T > {}, impl < T : Clone > Clone for NodeB < T >
           {
               fn clone(& self) -> Self
               {
                   NodeB
                   {
                       count : self.count, child_a : self.child_a.clone(), internal :
                       self.internal.clone(), phantom : self.phantom,
                   }
               }
           }, impl < T > std :: fmt :: Debug for NodeB < T >
           {
               fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt ::
               Result
               {
                   f.debug_struct("NodeB").field("count", &
                   self.count).field("child_a", &
                   "<child_a>").field("internal", & self.internal).finish()
               }
           }, impl < T > PartialEq for NodeB < T >
           {
               fn eq(& self, other : & Self) -> bool
               { self.count == other.count && self.internal == other.internal }
           }, unsafe impl < T > Send for NodeB < T > {}, unsafe impl < T > Sync for NodeB
           < T > {}, impl < T : Clone > Clone for NodeC < T >
           {
               fn clone(& self) -> Self
               {
                   NodeC
                   {
                       data : self.data, ref_a : self.ref_a.clone(), ref_b :
                       self.ref_b.clone(), phantom : self.phantom,
                   }
               }
           }, impl < T > std :: fmt :: Debug for NodeC < T >
           {
               fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt ::
               Result
               {
                   f.debug_struct("NodeC").field("data", &
                   self.data).field("ref_a", &
                   "<ref_a>").field("ref_b", & "<ref_b>").finish()
               }
           }, impl < T > PartialEq for NodeC < T >
           { fn eq(& self, other : & Self) -> bool { self.data == other.data } }, impl <
           T > std :: hash :: Hash for NodeC < T >
           {
               fn hash < H : std :: hash :: Hasher > (& self, state : & mut H)
               { self.data.hash(state); }
           }, impl < T > std :: hash :: Hash for NodeB < T >
           {
               fn hash < H : std :: hash :: Hasher > (& self, state : & mut H)
               { self.count.hash(state); }
           }, impl < T > Default for NodeA < T >
           {
               fn default() -> Self
               {
                   NodeA
                   { value : String :: new(), child_b : None, phantom : PhantomData, }
               }
           }, impl < T > Default for NodeB < T >
           {
               fn default() -> Self
               {
                   NodeB
                   {
                       count : 0, child_a : None, internal : InternalType(0.0), phantom :
                       PhantomData,
                   }
               }
           }, impl < T > Default for NodeC < T >
           {
               fn default() -> Self
               { NodeC { data : 0, ref_a : None, ref_b : None, phantom : PhantomData, } }
           }, impl < T > TestTrait for NodeA < T > where typedef :: generic_types ::
           Container < NodeB < T > , NodeC < T > > : LocalTrait,
           { fn test_method(& self) -> String { format! ("NodeA: {}", self.value) } },
           impl < T > LocalTrait for NodeB < T >
           {
               fn local_method(& self) -> usize
               { self.count + (self.internal.coinduction_method() as usize) }
           }, impl < T : 'static + Clone + Send > CircularTrait for NodeB < T > where T :
           Clone, typedef :: generic_types :: Wrapper < NodeA < T > > : LocalTrait,
           typedef :: generic_types :: MultiGeneric < NodeC < T > , NodeB < T > , String
           > : CircularTrait,
           {
               fn circular_method(& self) -> Box < dyn CircularTrait >
               {
                   Box ::
                   new(NodeC :: < T >
                   {
                       data : self.count as i32, ref_a : None, ref_b : None, phantom :
                       PhantomData,
                   })
               }
           }, impl < T : 'static + Clone + Send > CircularTrait for NodeC < T > where T :
           Clone, NodeA < T > : TestTrait, NodeB < T > : LocalTrait, typedef ::
           generic_types :: Container < NodeA < T > , NodeB < T > > : LocalTrait,
           {
               fn circular_method(& self) -> Box < dyn CircularTrait >
               {
                   Box ::
                   new(NodeA :: < T >
                   {
                       value : format! ("Generated from NodeC: {}", self.data), child_b :
                       None, phantom : PhantomData,
                   })
               }
           }, impl < T : Clone > CoinductionLocalTrait for NodeA < T > where typedef ::
           generic_types :: Wrapper < NodeB < T > > : LocalTrait,
           { fn coinduction_method(& self) -> f64 { self.value.len() as f64 } }, impl < T
           > CoinductionLocalTrait for NodeB < T >
           {
               fn coinduction_method(& self) -> f64
               { self.count as f64 * self.internal.0 }
           }, impl < T > CoinductionLocalTrait for NodeC < T > where NodeA < T > :
           TestTrait + CoinductionLocalTrait, NodeB < T > : LocalTrait +
           CoinductionLocalTrait,
           { fn coinduction_method(& self) -> f64 { self.data as f64 * 2.0 } }]"#
            && e.to == r#"typedef :: generic_types :: Container !
           {
               "0.2.0", None,
               [typedef :: generic_types :: Container < NodeB < T >, NodeC < T > > :
               LocalTrait, typedef :: generic_types :: MultiGeneric < NodeC < T > , NodeB
               < T > , String > : CircularTrait, typedef :: generic_types :: Wrapper <
               NodeB < T > > : LocalTrait, typedef :: generic_types :: Container < NodeB
               < T > , InternalType > : LocalTrait, typedef :: generic_types :: Wrapper <
               NodeA < T > > : LocalTrait, typedef :: generic_types :: Container < NodeA
               < T > , NodeB < T > > : LocalTrait], { :: coinduction },
               [LocalTrait, TestTrait, CircularTrait, CoinductionLocalTrait],
               [NodeC, NodeA, NodeB],
               [{
                   [],
                   [(NodeA < T > : CircularTrait, T : Clone),
                   (NodeA < T > : CircularTrait, T : Send),
                   (NodeA < T > : CircularTrait, NodeA < T > : Clone),
                   (NodeA < T > : CircularTrait, typedef :: generic_types :: Container <
                   NodeB < T > , InternalType > : LocalTrait),
                   (NodeA < T > : CircularTrait, typedef :: generic_types ::
                   ConstrainedStruct < std :: iter :: Once < NodeC < T > > > :
                   CircularTrait)], [T : 'static + Clone + Send]
               }, None, None, None, None, None, None, None, None, None, None, None, None,
               None, None, None, None, None, None, None,
               {
                   [],
                   [(NodeA < T > : TestTrait, typedef :: generic_types :: Container <
                   NodeB < T > , NodeC < T > > : LocalTrait)], [T]
               }, { [NodeB < T > : LocalTrait], [], [T] },
               {
                   [],
                   [(NodeB < T > : CircularTrait, T : Clone),
                   (NodeB < T > : CircularTrait, T : Send),
                   (NodeB < T > : CircularTrait, T : Clone),
                   (NodeB < T > : CircularTrait, typedef :: generic_types :: Wrapper <
                   NodeA < T > > : LocalTrait),
                   (NodeB < T > : CircularTrait, typedef :: generic_types :: MultiGeneric
                   < NodeC < T > , NodeB < T > , String > : CircularTrait)],
                   [T : 'static + Clone + Send]
               },
               {
                   [],
                   [(NodeC < T > : CircularTrait, T : Clone),
                   (NodeC < T > : CircularTrait, T : Send),
                   (NodeC < T > : CircularTrait, T : Clone),
                   (NodeC < T > : CircularTrait, NodeA < T > : TestTrait),
                   (NodeC < T > : CircularTrait, NodeB < T > : LocalTrait),
                   (NodeC < T > : CircularTrait, typedef :: generic_types :: Container <
                   NodeA < T > , NodeB < T > > : LocalTrait),
                   (NodeA < T > : TestTrait, typedef :: generic_types :: Container <
                   NodeB < T > , NodeC < T > > : LocalTrait)],
                   [T : 'static + Clone + Send]
               },
               {
                   [],
                   [(NodeA < T > : CoinductionLocalTrait, T : Clone),
                   (NodeA < T > : CoinductionLocalTrait, typedef :: generic_types ::
                   Wrapper < NodeB < T > > : LocalTrait)], [T : Clone]
               }, { [NodeB < T > : CoinductionLocalTrait], [], [T] },
               {
                   [],
                   [(NodeC < T > : CoinductionLocalTrait, NodeA < T > : TestTrait),
                   (NodeC < T > : CoinductionLocalTrait, NodeA < T > :
                   CoinductionLocalTrait),
                   (NodeC < T > : CoinductionLocalTrait, NodeB < T > : LocalTrait),
                   (NodeC < T > : CoinductionLocalTrait, NodeB < T > :
                   CoinductionLocalTrait),
                   (NodeA < T > : TestTrait, typedef :: generic_types :: Container <
                   NodeB < T > , NodeC < T > > : LocalTrait),
                   (NodeA < T > : CoinductionLocalTrait, T : Clone),
                   (NodeA < T > : CoinductionLocalTrait, typedef :: generic_types ::
                   Wrapper < NodeB < T > > : LocalTrait)], [T]
               }],
               [impl < T : 'static + Clone + Send > CircularTrait for NodeA < T > where
               NodeA < T > : Clone, typedef :: generic_types :: Container < NodeB < T > ,
               InternalType > : LocalTrait, typedef :: generic_types :: ConstrainedStruct
               < std :: iter :: Once < NodeC < T > > > : CircularTrait,
               {
                   fn circular_method(& self) -> Box < dyn CircularTrait >
                   {
                       Box ::
                       new(NodeB :: < T >
                       {
                           count : self.value.len(), child_a : None, internal :
                           InternalType(42.0), phantom : PhantomData,
                       })
                   }
               }, impl < T : Clone > Clone for NodeA < T >
               {
                   fn clone(& self) -> Self
                   {
                       NodeA
                       {
                           value : self.value.clone(), child_b : self.child_b.clone(),
                           phantom : self.phantom,
                       }
                   }
               }, impl < T > std :: fmt :: Debug for NodeA < T >
               {
                   fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt
                   :: Result
                   {
                       f.debug_struct("NodeA").field("value", &
                       self.value).field("child_b", & "<child_b>").finish()
                   }
               }, impl < T > PartialEq for NodeA < T >
               { fn eq(& self, other : & Self) -> bool { self.value == other.value } },
               impl < T > ToString for NodeA < T >
               { fn to_string(& self) -> String { format! ("NodeA({})", self.value) } },
               unsafe impl < T > Send for NodeA < T > {}, unsafe impl < T > Sync for
               NodeA < T > {}, impl < T : Clone > Clone for NodeB < T >
               {
                   fn clone(& self) -> Self
                   {
                       NodeB
                       {
                           count : self.count, child_a : self.child_a.clone(), internal :
                           self.internal.clone(), phantom : self.phantom,
                       }
                   }
               }, impl < T > std :: fmt :: Debug for NodeB < T >
               {
                   fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt
                   :: Result
                   {
                       f.debug_struct("NodeB").field("count", &
                       self.count).field("child_a", &
                       "<child_a>").field("internal", & self.internal).finish()
                   }
               }, impl < T > PartialEq for NodeB < T >
               {
                   fn eq(& self, other : & Self) -> bool
                   { self.count == other.count && self.internal == other.internal }
               }, unsafe impl < T > Send for NodeB < T > {}, unsafe impl < T > Sync for
               NodeB < T > {}, impl < T : Clone > Clone for NodeC < T >
               {
                   fn clone(& self) -> Self
                   {
                       NodeC
                       {
                           data : self.data, ref_a : self.ref_a.clone(), ref_b :
                           self.ref_b.clone(), phantom : self.phantom,
                       }
                   }
               }, impl < T > std :: fmt :: Debug for NodeC < T >
               {
                   fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt
                   :: Result
                   {
                       f.debug_struct("NodeC").field("data", &
                       self.data).field("ref_a", &
                       "<ref_a>").field("ref_b", & "<ref_b>").finish()
                   }
               }, impl < T > PartialEq for NodeC < T >
               { fn eq(& self, other : & Self) -> bool { self.data == other.data } },
               impl < T > std :: hash :: Hash for NodeC < T >
               {
                   fn hash < H : std :: hash :: Hasher > (& self, state : & mut H)
                   { self.data.hash(state); }
               }, impl < T > std :: hash :: Hash for NodeB < T >
               {
                   fn hash < H : std :: hash :: Hasher > (& self, state : & mut H)
                   { self.count.hash(state); }
               }, impl < T > Default for NodeA < T >
               {
                   fn default() -> Self
                   {
                       NodeA
                       {
                           value : String :: new(), child_b : None, phantom :
                           PhantomData,
                       }
                   }
               }, impl < T > Default for NodeB < T >
               {
                   fn default() -> Self
                   {
                       NodeB
                       {
                           count : 0, child_a : None, internal : InternalType(0.0),
                           phantom : PhantomData,
                       }
                   }
               }, impl < T > Default for NodeC < T >
               {
                   fn default() -> Self
                   {
                       NodeC
                       { data : 0, ref_a : None, ref_b : None, phantom : PhantomData, }
                   }
               }, impl < T > TestTrait for NodeA < T > where typedef :: generic_types ::
               Container < NodeB < T > , NodeC < T > > : LocalTrait,
               {
                   fn test_method(& self) -> String { format! ("NodeA: {}", self.value) }
               }, impl < T > LocalTrait for NodeB < T >
               {
                   fn local_method(& self) -> usize
                   { self.count + (self.internal.coinduction_method() as usize) }
               }, impl < T : 'static + Clone + Send > CircularTrait for NodeB < T > where
               T : Clone, typedef :: generic_types :: Wrapper < NodeA < T > > :
               LocalTrait, typedef :: generic_types :: MultiGeneric < NodeC < T > , NodeB
               < T > , String > : CircularTrait,
               {
                   fn circular_method(& self) -> Box < dyn CircularTrait >
                   {
                       Box ::
                       new(NodeC :: < T >
                       {
                           data : self.count as i32, ref_a : None, ref_b : None, phantom
                           : PhantomData,
                       })
                   }
               }, impl < T : 'static + Clone + Send > CircularTrait for NodeC < T > where
               T : Clone, NodeA < T > : TestTrait, NodeB < T > : LocalTrait, typedef ::
               generic_types :: Container < NodeA < T > , NodeB < T > > : LocalTrait,
               {
                   fn circular_method(& self) -> Box < dyn CircularTrait >
                   {
                       Box ::
                       new(NodeA :: < T >
                       {
                           value : format! ("Generated from NodeC: {}", self.data),
                           child_b : None, phantom : PhantomData,
                       })
                   }
               }, impl < T : Clone > CoinductionLocalTrait for NodeA < T > where typedef
               :: generic_types :: Wrapper < NodeB < T > > : LocalTrait,
               { fn coinduction_method(& self) -> f64 { self.value.len() as f64 } }, impl
               < T > CoinductionLocalTrait for NodeB < T >
               {
                   fn coinduction_method(& self) -> f64
                   { self.count as f64 * self.internal.0 }
               }, impl < T > CoinductionLocalTrait for NodeC < T > where NodeA < T > :
               TestTrait + CoinductionLocalTrait, NodeB < T > : LocalTrait +
               CoinductionLocalTrait,
               { fn coinduction_method(& self) -> f64 { self.data as f64 * 2.0 } }]
           }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__MultiGeneric_temporal_13753066654786479059"
            && e.input == r#""0.2.0", None,
           [typedef :: generic_types :: MultiGeneric < NodeC < T >, NodeB < T >, String >
           : CircularTrait, typedef :: generic_types :: Wrapper < NodeB < T > > :
           LocalTrait, typedef :: generic_types :: Container < NodeB < T > , InternalType
           > : LocalTrait, typedef :: generic_types :: Wrapper < NodeA < T > > :
           LocalTrait, typedef :: generic_types :: Container < NodeA < T > , NodeB < T >
           > : LocalTrait], { :: coinduction },
           [LocalTrait, TestTrait, CircularTrait, CoinductionLocalTrait],
           [NodeA, NodeC, NodeB],
           [{
               [],
               [(NodeA < T > : CircularTrait, T : Clone),
               (NodeA < T > : CircularTrait, T : Send),
               (NodeA < T > : CircularTrait, NodeA < T > : Clone),
               (NodeA < T > : CircularTrait, typedef :: generic_types :: Container <
               NodeB < T > , InternalType > : LocalTrait),
               (NodeA < T > : CircularTrait, typedef :: generic_types ::
               ConstrainedStruct < std :: iter :: Once < NodeC < T > > > :
               CircularTrait)], [T : 'static + Clone + Send]
           }, None, None, None, None, None, None, None, None, None, None, None, None,
           None, None, None, None, None, None, None,
           {
               [],
               [(NodeA < T > : TestTrait, typedef :: generic_types :: Container < NodeB <
               T > , NodeC < T > > : LocalTrait)], [T]
           }, { [NodeB < T > : LocalTrait], [], [T] },
           {
               [],
               [(NodeB < T > : CircularTrait, T : Clone),
               (NodeB < T > : CircularTrait, T : Send),
               (NodeB < T > : CircularTrait, T : Clone),
               (NodeB < T > : CircularTrait, typedef :: generic_types :: Wrapper < NodeA
               < T > > : LocalTrait),
               (NodeB < T > : CircularTrait, typedef :: generic_types :: MultiGeneric <
               NodeC < T > , NodeB < T > , String > : CircularTrait)],
               [T : 'static + Clone + Send]
           },
           {
               [],
               [(NodeC < T > : CircularTrait, T : Clone),
               (NodeC < T > : CircularTrait, T : Send),
               (NodeC < T > : CircularTrait, T : Clone),
               (NodeC < T > : CircularTrait, NodeA < T > : TestTrait),
               (NodeC < T > : CircularTrait, NodeB < T > : LocalTrait),
               (NodeC < T > : CircularTrait, typedef :: generic_types :: Container <
               NodeA < T > , NodeB < T > > : LocalTrait),
               (NodeA < T > : TestTrait, typedef :: generic_types :: Container < NodeB <
               T > , NodeC < T > > : LocalTrait)], [T : 'static + Clone + Send]
           },
           {
               [],
               [(NodeA < T > : CoinductionLocalTrait, T : Clone),
               (NodeA < T > : CoinductionLocalTrait, typedef :: generic_types :: Wrapper
               < NodeB < T > > : LocalTrait)], [T : Clone]
           }, { [NodeB < T > : CoinductionLocalTrait], [], [T] },
           {
               [],
               [(NodeC < T > : CoinductionLocalTrait, NodeA < T > : TestTrait),
               (NodeC < T > : CoinductionLocalTrait, NodeA < T > :
               CoinductionLocalTrait),
               (NodeC < T > : CoinductionLocalTrait, NodeB < T > : LocalTrait),
               (NodeC < T > : CoinductionLocalTrait, NodeB < T > :
               CoinductionLocalTrait),
               (NodeA < T > : TestTrait, typedef :: generic_types :: Container < NodeB <
               T > , NodeC < T > > : LocalTrait),
               (NodeA < T > : CoinductionLocalTrait, T : Clone),
               (NodeA < T > : CoinductionLocalTrait, typedef :: generic_types :: Wrapper
               < NodeB < T > > : LocalTrait)], [T]
           }],
           [impl < T : 'static + Clone + Send > CircularTrait for NodeA < T > where NodeA
           < T > : Clone, typedef :: generic_types :: Container < NodeB < T > ,
           InternalType > : LocalTrait, typedef :: generic_types :: ConstrainedStruct <
           std :: iter :: Once < NodeC < T > > > : CircularTrait,
           {
               fn circular_method(& self) -> Box < dyn CircularTrait >
               {
                   Box ::
                   new(NodeB :: < T >
                   {
                       count : self.value.len(), child_a : None, internal :
                       InternalType(42.0), phantom : PhantomData,
                   })
               }
           }, impl < T : Clone > Clone for NodeA < T >
           {
               fn clone(& self) -> Self
               {
                   NodeA
                   {
                       value : self.value.clone(), child_b : self.child_b.clone(),
                       phantom : self.phantom,
                   }
               }
           }, impl < T > std :: fmt :: Debug for NodeA < T >
           {
               fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt ::
               Result
               {
                   f.debug_struct("NodeA").field("value", &
                   self.value).field("child_b", & "<child_b>").finish()
               }
           }, impl < T > PartialEq for NodeA < T >
           { fn eq(& self, other : & Self) -> bool { self.value == other.value } }, impl
           < T > ToString for NodeA < T >
           { fn to_string(& self) -> String { format! ("NodeA({})", self.value) } },
           unsafe impl < T > Send for NodeA < T > {}, unsafe impl < T > Sync for NodeA <
           T > {}, impl < T : Clone > Clone for NodeB < T >
           {
               fn clone(& self) -> Self
               {
                   NodeB
                   {
                       count : self.count, child_a : self.child_a.clone(), internal :
                       self.internal.clone(), phantom : self.phantom,
                   }
               }
           }, impl < T > std :: fmt :: Debug for NodeB < T >
           {
               fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt ::
               Result
               {
                   f.debug_struct("NodeB").field("count", &
                   self.count).field("child_a", &
                   "<child_a>").field("internal", & self.internal).finish()
               }
           }, impl < T > PartialEq for NodeB < T >
           {
               fn eq(& self, other : & Self) -> bool
               { self.count == other.count && self.internal == other.internal }
           }, unsafe impl < T > Send for NodeB < T > {}, unsafe impl < T > Sync for NodeB
           < T > {}, impl < T : Clone > Clone for NodeC < T >
           {
               fn clone(& self) -> Self
               {
                   NodeC
                   {
                       data : self.data, ref_a : self.ref_a.clone(), ref_b :
                       self.ref_b.clone(), phantom : self.phantom,
                   }
               }
           }, impl < T > std :: fmt :: Debug for NodeC < T >
           {
               fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt ::
               Result
               {
                   f.debug_struct("NodeC").field("data", &
                   self.data).field("ref_a", &
                   "<ref_a>").field("ref_b", & "<ref_b>").finish()
               }
           }, impl < T > PartialEq for NodeC < T >
           { fn eq(& self, other : & Self) -> bool { self.data == other.data } }, impl <
           T > std :: hash :: Hash for NodeC < T >
           {
               fn hash < H : std :: hash :: Hasher > (& self, state : & mut H)
               { self.data.hash(state); }
           }, impl < T > std :: hash :: Hash for NodeB < T >
           {
               fn hash < H : std :: hash :: Hasher > (& self, state : & mut H)
               { self.count.hash(state); }
           }, impl < T > Default for NodeA < T >
           {
               fn default() -> Self
               {
                   NodeA
                   { value : String :: new(), child_b : None, phantom : PhantomData, }
               }
           }, impl < T > Default for NodeB < T >
           {
               fn default() -> Self
               {
                   NodeB
                   {
                       count : 0, child_a : None, internal : InternalType(0.0), phantom :
                       PhantomData,
                   }
               }
           }, impl < T > Default for NodeC < T >
           {
               fn default() -> Self
               { NodeC { data : 0, ref_a : None, ref_b : None, phantom : PhantomData, } }
           }, impl < T > TestTrait for NodeA < T > where typedef :: generic_types ::
           Container < NodeB < T > , NodeC < T > > : LocalTrait,
           { fn test_method(& self) -> String { format! ("NodeA: {}", self.value) } },
           impl < T > LocalTrait for NodeB < T >
           {
               fn local_method(& self) -> usize
               { self.count + (self.internal.coinduction_method() as usize) }
           }, impl < T : 'static + Clone + Send > CircularTrait for NodeB < T > where T :
           Clone, typedef :: generic_types :: Wrapper < NodeA < T > > : LocalTrait,
           typedef :: generic_types :: MultiGeneric < NodeC < T > , NodeB < T > , String
           > : CircularTrait,
           {
               fn circular_method(& self) -> Box < dyn CircularTrait >
               {
                   Box ::
                   new(NodeC :: < T >
                   {
                       data : self.count as i32, ref_a : None, ref_b : None, phantom :
                       PhantomData,
                   })
               }
           }, impl < T : 'static + Clone + Send > CircularTrait for NodeC < T > where T :
           Clone, NodeA < T > : TestTrait, NodeB < T > : LocalTrait, typedef ::
           generic_types :: Container < NodeA < T > , NodeB < T > > : LocalTrait,
           {
               fn circular_method(& self) -> Box < dyn CircularTrait >
               {
                   Box ::
                   new(NodeA :: < T >
                   {
                       value : format! ("Generated from NodeC: {}", self.data), child_b :
                       None, phantom : PhantomData,
                   })
               }
           }, impl < T : Clone > CoinductionLocalTrait for NodeA < T > where typedef ::
           generic_types :: Wrapper < NodeB < T > > : LocalTrait,
           { fn coinduction_method(& self) -> f64 { self.value.len() as f64 } }, impl < T
           > CoinductionLocalTrait for NodeB < T >
           {
               fn coinduction_method(& self) -> f64
               { self.count as f64 * self.internal.0 }
           }, impl < T > CoinductionLocalTrait for NodeC < T > where NodeA < T > :
           TestTrait + CoinductionLocalTrait, NodeB < T > : LocalTrait +
           CoinductionLocalTrait,
           { fn coinduction_method(& self) -> f64 { self.data as f64 * 2.0 } }]"#
            && e.to == r#":: coinduction :: __next_step!
           {
               "0.2.0", Typedef
               {
                   predicates :
                   [([__T_11_13753066654786479059, __U_11_13753066654786479059,
                   __V_11_13753066654786479059], MultiGeneric <
                   __T_11_13753066654786479059, __U_11_13753066654786479059,
                   __V_11_13753066654786479059 > : CircularTrait,
                   [__T_11_13753066654786479059 : Clone, __T_11_13753066654786479059 : ::
                   std :: fmt :: Debug, __T_11_13753066654786479059 : Send,
                   __U_11_13753066654786479059 : Send, __U_11_13753066654786479059 :
                   Sync, __U_11_13753066654786479059 : Default,
                   __V_11_13753066654786479059 : :: std :: fmt :: Debug,
                   __V_11_13753066654786479059 : Hash, __V_11_13753066654786479059 :
                   Clone]),
                   ([__T_14_13753066654786479059, __U_14_13753066654786479059,
                   __V_14_13753066654786479059], MultiGeneric <
                   __T_14_13753066654786479059, __U_14_13753066654786479059,
                   __V_14_13753066654786479059 > : ExtendedTrait,
                   [__T_14_13753066654786479059 : Clone, __T_14_13753066654786479059 :
                   PartialOrd, __U_14_13753066654786479059 : Send,
                   __U_14_13753066654786479059 : Sync, __U_14_13753066654786479059 :
                   Clone, __V_14_13753066654786479059 : :: std :: fmt :: Debug,
                   __V_14_13753066654786479059 : Hash, __V_14_13753066654786479059 :
                   Default])]
               },
               [typedef :: generic_types :: MultiGeneric < NodeC < T >, NodeB < T >,
               String > : CircularTrait, typedef :: generic_types :: Wrapper < NodeB < T
               > > : LocalTrait, typedef :: generic_types :: Container < NodeB < T > ,
               InternalType > : LocalTrait, typedef :: generic_types :: Wrapper < NodeA <
               T > > : LocalTrait, typedef :: generic_types :: Container < NodeA < T > ,
               NodeB < T > > : LocalTrait], { :: coinduction },
               [LocalTrait, TestTrait, CircularTrait, CoinductionLocalTrait],
               [NodeA, NodeC, NodeB],
               [{
                   [],
                   [(NodeA < T > : CircularTrait, T : Clone),
                   (NodeA < T > : CircularTrait, T : Send),
                   (NodeA < T > : CircularTrait, NodeA < T > : Clone),
                   (NodeA < T > : CircularTrait, typedef :: generic_types :: Container <
                   NodeB < T > , InternalType > : LocalTrait),
                   (NodeA < T > : CircularTrait, typedef :: generic_types ::
                   ConstrainedStruct < std :: iter :: Once < NodeC < T > > > :
                   CircularTrait)], [T : 'static + Clone + Send]
               }, None, None, None, None, None, None, None, None, None, None, None, None,
               None, None, None, None, None, None, None,
               {
                   [],
                   [(NodeA < T > : TestTrait, typedef :: generic_types :: Container <
                   NodeB < T > , NodeC < T > > : LocalTrait)], [T]
               }, { [NodeB < T > : LocalTrait], [], [T] },
               {
                   [],
                   [(NodeB < T > : CircularTrait, T : Clone),
                   (NodeB < T > : CircularTrait, T : Send),
                   (NodeB < T > : CircularTrait, T : Clone),
                   (NodeB < T > : CircularTrait, typedef :: generic_types :: Wrapper <
                   NodeA < T > > : LocalTrait),
                   (NodeB < T > : CircularTrait, typedef :: generic_types :: MultiGeneric
                   < NodeC < T > , NodeB < T > , String > : CircularTrait)],
                   [T : 'static + Clone + Send]
               },
               {
                   [],
                   [(NodeC < T > : CircularTrait, T : Clone),
                   (NodeC < T > : CircularTrait, T : Send),
                   (NodeC < T > : CircularTrait, T : Clone),
                   (NodeC < T > : CircularTrait, NodeA < T > : TestTrait),
                   (NodeC < T > : CircularTrait, NodeB < T > : LocalTrait),
                   (NodeC < T > : CircularTrait, typedef :: generic_types :: Container <
                   NodeA < T > , NodeB < T > > : LocalTrait),
                   (NodeA < T > : TestTrait, typedef :: generic_types :: Container <
                   NodeB < T > , NodeC < T > > : LocalTrait)],
                   [T : 'static + Clone + Send]
               },
               {
                   [],
                   [(NodeA < T > : CoinductionLocalTrait, T : Clone),
                   (NodeA < T > : CoinductionLocalTrait, typedef :: generic_types ::
                   Wrapper < NodeB < T > > : LocalTrait)], [T : Clone]
               }, { [NodeB < T > : CoinductionLocalTrait], [], [T] },
               {
                   [],
                   [(NodeC < T > : CoinductionLocalTrait, NodeA < T > : TestTrait),
                   (NodeC < T > : CoinductionLocalTrait, NodeA < T > :
                   CoinductionLocalTrait),
                   (NodeC < T > : CoinductionLocalTrait, NodeB < T > : LocalTrait),
                   (NodeC < T > : CoinductionLocalTrait, NodeB < T > :
                   CoinductionLocalTrait),
                   (NodeA < T > : TestTrait, typedef :: generic_types :: Container <
                   NodeB < T > , NodeC < T > > : LocalTrait),
                   (NodeA < T > : CoinductionLocalTrait, T : Clone),
                   (NodeA < T > : CoinductionLocalTrait, typedef :: generic_types ::
                   Wrapper < NodeB < T > > : LocalTrait)], [T]
               }],
               [impl < T : 'static + Clone + Send > CircularTrait for NodeA < T > where
               NodeA < T > : Clone, typedef :: generic_types :: Container < NodeB < T > ,
               InternalType > : LocalTrait, typedef :: generic_types :: ConstrainedStruct
               < std :: iter :: Once < NodeC < T > > > : CircularTrait,
               {
                   fn circular_method(& self) -> Box < dyn CircularTrait >
                   {
                       Box ::
                       new(NodeB :: < T >
                       {
                           count : self.value.len(), child_a : None, internal :
                           InternalType(42.0), phantom : PhantomData,
                       })
                   }
               }, impl < T : Clone > Clone for NodeA < T >
               {
                   fn clone(& self) -> Self
                   {
                       NodeA
                       {
                           value : self.value.clone(), child_b : self.child_b.clone(),
                           phantom : self.phantom,
                       }
                   }
               }, impl < T > std :: fmt :: Debug for NodeA < T >
               {
                   fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt
                   :: Result
                   {
                       f.debug_struct("NodeA").field("value", &
                       self.value).field("child_b", & "<child_b>").finish()
                   }
               }, impl < T > PartialEq for NodeA < T >
               { fn eq(& self, other : & Self) -> bool { self.value == other.value } },
               impl < T > ToString for NodeA < T >
               { fn to_string(& self) -> String { format! ("NodeA({})", self.value) } },
               unsafe impl < T > Send for NodeA < T > {}, unsafe impl < T > Sync for
               NodeA < T > {}, impl < T : Clone > Clone for NodeB < T >
               {
                   fn clone(& self) -> Self
                   {
                       NodeB
                       {
                           count : self.count, child_a : self.child_a.clone(), internal :
                           self.internal.clone(), phantom : self.phantom,
                       }
                   }
               }, impl < T > std :: fmt :: Debug for NodeB < T >
               {
                   fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt
                   :: Result
                   {
                       f.debug_struct("NodeB").field("count", &
                       self.count).field("child_a", &
                       "<child_a>").field("internal", & self.internal).finish()
                   }
               }, impl < T > PartialEq for NodeB < T >
               {
                   fn eq(& self, other : & Self) -> bool
                   { self.count == other.count && self.internal == other.internal }
               }, unsafe impl < T > Send for NodeB < T > {}, unsafe impl < T > Sync for
               NodeB < T > {}, impl < T : Clone > Clone for NodeC < T >
               {
                   fn clone(& self) -> Self
                   {
                       NodeC
                       {
                           data : self.data, ref_a : self.ref_a.clone(), ref_b :
                           self.ref_b.clone(), phantom : self.phantom,
                       }
                   }
               }, impl < T > std :: fmt :: Debug for NodeC < T >
               {
                   fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt
                   :: Result
                   {
                       f.debug_struct("NodeC").field("data", &
                       self.data).field("ref_a", &
                       "<ref_a>").field("ref_b", & "<ref_b>").finish()
                   }
               }, impl < T > PartialEq for NodeC < T >
               { fn eq(& self, other : & Self) -> bool { self.data == other.data } },
               impl < T > std :: hash :: Hash for NodeC < T >
               {
                   fn hash < H : std :: hash :: Hasher > (& self, state : & mut H)
                   { self.data.hash(state); }
               }, impl < T > std :: hash :: Hash for NodeB < T >
               {
                   fn hash < H : std :: hash :: Hasher > (& self, state : & mut H)
                   { self.count.hash(state); }
               }, impl < T > Default for NodeA < T >
               {
                   fn default() -> Self
                   {
                       NodeA
                       {
                           value : String :: new(), child_b : None, phantom :
                           PhantomData,
                       }
                   }
               }, impl < T > Default for NodeB < T >
               {
                   fn default() -> Self
                   {
                       NodeB
                       {
                           count : 0, child_a : None, internal : InternalType(0.0),
                           phantom : PhantomData,
                       }
                   }
               }, impl < T > Default for NodeC < T >
               {
                   fn default() -> Self
                   {
                       NodeC
                       { data : 0, ref_a : None, ref_b : None, phantom : PhantomData, }
                   }
               }, impl < T > TestTrait for NodeA < T > where typedef :: generic_types ::
               Container < NodeB < T > , NodeC < T > > : LocalTrait,
               {
                   fn test_method(& self) -> String { format! ("NodeA: {}", self.value) }
               }, impl < T > LocalTrait for NodeB < T >
               {
                   fn local_method(& self) -> usize
                   { self.count + (self.internal.coinduction_method() as usize) }
               }, impl < T : 'static + Clone + Send > CircularTrait for NodeB < T > where
               T : Clone, typedef :: generic_types :: Wrapper < NodeA < T > > :
               LocalTrait, typedef :: generic_types :: MultiGeneric < NodeC < T > , NodeB
               < T > , String > : CircularTrait,
               {
                   fn circular_method(& self) -> Box < dyn CircularTrait >
                   {
                       Box ::
                       new(NodeC :: < T >
                       {
                           data : self.count as i32, ref_a : None, ref_b : None, phantom
                           : PhantomData,
                       })
                   }
               }, impl < T : 'static + Clone + Send > CircularTrait for NodeC < T > where
               T : Clone, NodeA < T > : TestTrait, NodeB < T > : LocalTrait, typedef ::
               generic_types :: Container < NodeA < T > , NodeB < T > > : LocalTrait,
               {
                   fn circular_method(& self) -> Box < dyn CircularTrait >
                   {
                       Box ::
                       new(NodeA :: < T >
                       {
                           value : format! ("Generated from NodeC: {}", self.data),
                           child_b : None, phantom : PhantomData,
                       })
                   }
               }, impl < T : Clone > CoinductionLocalTrait for NodeA < T > where typedef
               :: generic_types :: Wrapper < NodeB < T > > : LocalTrait,
               { fn coinduction_method(& self) -> f64 { self.value.len() as f64 } }, impl
               < T > CoinductionLocalTrait for NodeB < T >
               {
                   fn coinduction_method(& self) -> f64
                   { self.count as f64 * self.internal.0 }
               }, impl < T > CoinductionLocalTrait for NodeC < T > where NodeA < T > :
               TestTrait + CoinductionLocalTrait, NodeB < T > : LocalTrait +
               CoinductionLocalTrait,
               { fn coinduction_method(& self) -> f64 { self.data as f64 * 2.0 } }]
           }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__Wrapper_temporal_13753066654786479059"
            && e.input == r#""0.2.0", None,
           [typedef :: generic_types :: Wrapper < NodeB < T > > : LocalTrait, typedef ::
           generic_types :: Container < NodeB < T > , InternalType > : LocalTrait,
           typedef :: generic_types :: Wrapper < NodeA < T > > : LocalTrait, typedef ::
           generic_types :: Container < NodeA < T > , NodeB < T > > : LocalTrait],
           { :: coinduction },
           [LocalTrait, TestTrait, CircularTrait, CoinductionLocalTrait],
           [NodeC, NodeB, NodeA],
           [{
               [],
               [(NodeA < T > : CircularTrait, T : Clone),
               (NodeA < T > : CircularTrait, T : Send),
               (NodeA < T > : CircularTrait, NodeA < T > : Clone),
               (NodeA < T > : CircularTrait, typedef :: generic_types :: Container <
               NodeB < T > , InternalType > : LocalTrait),
               (NodeA < T > : CircularTrait, typedef :: generic_types ::
               ConstrainedStruct < std :: iter :: Once < NodeC < T > > > :
               CircularTrait)], [T : 'static + Clone + Send]
           }, None, None, None, None, None, None, None, None, None, None, None, None,
           None, None, None, None, None, None, None,
           {
               [],
               [(NodeA < T > : TestTrait, typedef :: generic_types :: Container < NodeB <
               T > , NodeC < T > > : LocalTrait)], [T]
           }, { [NodeB < T > : LocalTrait], [], [T] },
           {
               [],
               [(NodeB < T > : CircularTrait, T : Clone),
               (NodeB < T > : CircularTrait, T : Send),
               (NodeB < T > : CircularTrait, T : Clone),
               (NodeB < T > : CircularTrait, typedef :: generic_types :: Wrapper < NodeA
               < T > > : LocalTrait),
               (NodeB < T > : CircularTrait, typedef :: generic_types :: MultiGeneric <
               NodeC < T > , NodeB < T > , String > : CircularTrait)],
               [T : 'static + Clone + Send]
           },
           {
               [],
               [(NodeC < T > : CircularTrait, T : Clone),
               (NodeC < T > : CircularTrait, T : Send),
               (NodeC < T > : CircularTrait, T : Clone),
               (NodeC < T > : CircularTrait, NodeA < T > : TestTrait),
               (NodeC < T > : CircularTrait, NodeB < T > : LocalTrait),
               (NodeC < T > : CircularTrait, typedef :: generic_types :: Container <
               NodeA < T > , NodeB < T > > : LocalTrait),
               (NodeA < T > : TestTrait, typedef :: generic_types :: Container < NodeB <
               T > , NodeC < T > > : LocalTrait)], [T : 'static + Clone + Send]
           },
           {
               [],
               [(NodeA < T > : CoinductionLocalTrait, T : Clone),
               (NodeA < T > : CoinductionLocalTrait, typedef :: generic_types :: Wrapper
               < NodeB < T > > : LocalTrait)], [T : Clone]
           }, { [NodeB < T > : CoinductionLocalTrait], [], [T] },
           {
               [],
               [(NodeC < T > : CoinductionLocalTrait, NodeA < T > : TestTrait),
               (NodeC < T > : CoinductionLocalTrait, NodeA < T > :
               CoinductionLocalTrait),
               (NodeC < T > : CoinductionLocalTrait, NodeB < T > : LocalTrait),
               (NodeC < T > : CoinductionLocalTrait, NodeB < T > :
               CoinductionLocalTrait),
               (NodeA < T > : TestTrait, typedef :: generic_types :: Container < NodeB <
               T > , NodeC < T > > : LocalTrait),
               (NodeA < T > : CoinductionLocalTrait, T : Clone),
               (NodeA < T > : CoinductionLocalTrait, typedef :: generic_types :: Wrapper
               < NodeB < T > > : LocalTrait)], [T]
           }],
           [impl < T : 'static + Clone + Send > CircularTrait for NodeA < T > where NodeA
           < T > : Clone, typedef :: generic_types :: Container < NodeB < T > ,
           InternalType > : LocalTrait, typedef :: generic_types :: ConstrainedStruct <
           std :: iter :: Once < NodeC < T > > > : CircularTrait,
           {
               fn circular_method(& self) -> Box < dyn CircularTrait >
               {
                   Box ::
                   new(NodeB :: < T >
                   {
                       count : self.value.len(), child_a : None, internal :
                       InternalType(42.0), phantom : PhantomData,
                   })
               }
           }, impl < T : Clone > Clone for NodeA < T >
           {
               fn clone(& self) -> Self
               {
                   NodeA
                   {
                       value : self.value.clone(), child_b : self.child_b.clone(),
                       phantom : self.phantom,
                   }
               }
           }, impl < T > std :: fmt :: Debug for NodeA < T >
           {
               fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt ::
               Result
               {
                   f.debug_struct("NodeA").field("value", &
                   self.value).field("child_b", & "<child_b>").finish()
               }
           }, impl < T > PartialEq for NodeA < T >
           { fn eq(& self, other : & Self) -> bool { self.value == other.value } }, impl
           < T > ToString for NodeA < T >
           { fn to_string(& self) -> String { format! ("NodeA({})", self.value) } },
           unsafe impl < T > Send for NodeA < T > {}, unsafe impl < T > Sync for NodeA <
           T > {}, impl < T : Clone > Clone for NodeB < T >
           {
               fn clone(& self) -> Self
               {
                   NodeB
                   {
                       count : self.count, child_a : self.child_a.clone(), internal :
                       self.internal.clone(), phantom : self.phantom,
                   }
               }
           }, impl < T > std :: fmt :: Debug for NodeB < T >
           {
               fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt ::
               Result
               {
                   f.debug_struct("NodeB").field("count", &
                   self.count).field("child_a", &
                   "<child_a>").field("internal", & self.internal).finish()
               }
           }, impl < T > PartialEq for NodeB < T >
           {
               fn eq(& self, other : & Self) -> bool
               { self.count == other.count && self.internal == other.internal }
           }, unsafe impl < T > Send for NodeB < T > {}, unsafe impl < T > Sync for NodeB
           < T > {}, impl < T : Clone > Clone for NodeC < T >
           {
               fn clone(& self) -> Self
               {
                   NodeC
                   {
                       data : self.data, ref_a : self.ref_a.clone(), ref_b :
                       self.ref_b.clone(), phantom : self.phantom,
                   }
               }
           }, impl < T > std :: fmt :: Debug for NodeC < T >
           {
               fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt ::
               Result
               {
                   f.debug_struct("NodeC").field("data", &
                   self.data).field("ref_a", &
                   "<ref_a>").field("ref_b", & "<ref_b>").finish()
               }
           }, impl < T > PartialEq for NodeC < T >
           { fn eq(& self, other : & Self) -> bool { self.data == other.data } }, impl <
           T > std :: hash :: Hash for NodeC < T >
           {
               fn hash < H : std :: hash :: Hasher > (& self, state : & mut H)
               { self.data.hash(state); }
           }, impl < T > std :: hash :: Hash for NodeB < T >
           {
               fn hash < H : std :: hash :: Hasher > (& self, state : & mut H)
               { self.count.hash(state); }
           }, impl < T > Default for NodeA < T >
           {
               fn default() -> Self
               {
                   NodeA
                   { value : String :: new(), child_b : None, phantom : PhantomData, }
               }
           }, impl < T > Default for NodeB < T >
           {
               fn default() -> Self
               {
                   NodeB
                   {
                       count : 0, child_a : None, internal : InternalType(0.0), phantom :
                       PhantomData,
                   }
               }
           }, impl < T > Default for NodeC < T >
           {
               fn default() -> Self
               { NodeC { data : 0, ref_a : None, ref_b : None, phantom : PhantomData, } }
           }, impl < T > TestTrait for NodeA < T > where typedef :: generic_types ::
           Container < NodeB < T > , NodeC < T > > : LocalTrait,
           { fn test_method(& self) -> String { format! ("NodeA: {}", self.value) } },
           impl < T > LocalTrait for NodeB < T >
           {
               fn local_method(& self) -> usize
               { self.count + (self.internal.coinduction_method() as usize) }
           }, impl < T : 'static + Clone + Send > CircularTrait for NodeB < T > where T :
           Clone, typedef :: generic_types :: Wrapper < NodeA < T > > : LocalTrait,
           typedef :: generic_types :: MultiGeneric < NodeC < T > , NodeB < T > , String
           > : CircularTrait,
           {
               fn circular_method(& self) -> Box < dyn CircularTrait >
               {
                   Box ::
                   new(NodeC :: < T >
                   {
                       data : self.count as i32, ref_a : None, ref_b : None, phantom :
                       PhantomData,
                   })
               }
           }, impl < T : 'static + Clone + Send > CircularTrait for NodeC < T > where T :
           Clone, NodeA < T > : TestTrait, NodeB < T > : LocalTrait, typedef ::
           generic_types :: Container < NodeA < T > , NodeB < T > > : LocalTrait,
           {
               fn circular_method(& self) -> Box < dyn CircularTrait >
               {
                   Box ::
                   new(NodeA :: < T >
                   {
                       value : format! ("Generated from NodeC: {}", self.data), child_b :
                       None, phantom : PhantomData,
                   })
               }
           }, impl < T : Clone > CoinductionLocalTrait for NodeA < T > where typedef ::
           generic_types :: Wrapper < NodeB < T > > : LocalTrait,
           { fn coinduction_method(& self) -> f64 { self.value.len() as f64 } }, impl < T
           > CoinductionLocalTrait for NodeB < T >
           {
               fn coinduction_method(& self) -> f64
               { self.count as f64 * self.internal.0 }
           }, impl < T > CoinductionLocalTrait for NodeC < T > where NodeA < T > :
           TestTrait + CoinductionLocalTrait, NodeB < T > : LocalTrait +
           CoinductionLocalTrait,
           { fn coinduction_method(& self) -> f64 { self.data as f64 * 2.0 } }]"#
            && e.to == r#":: coinduction :: __next_step!
           {
               "0.2.0", Typedef
               {
                   predicates :
                   [([__T_8_13753066654786479059], Wrapper < __T_8_13753066654786479059 >
                   : TestTrait,
                   [__T_8_13753066654786479059 : Clone, __T_8_13753066654786479059 :
                   Debug, __T_8_13753066654786479059 : ToString]),
                   ([__T_10_13753066654786479059], Wrapper < __T_10_13753066654786479059
                   > : LocalTrait,
                   [__T_10_13753066654786479059 : Clone, __T_10_13753066654786479059 : ::
                   std :: fmt :: Debug, __T_10_13753066654786479059 : Default])]
               },
               [typedef :: generic_types :: Wrapper < NodeB < T > > : LocalTrait, typedef
               :: generic_types :: Container < NodeB < T > , InternalType > : LocalTrait,
               typedef :: generic_types :: Wrapper < NodeA < T > > : LocalTrait, typedef
               :: generic_types :: Container < NodeA < T > , NodeB < T > > : LocalTrait],
               { :: coinduction },
               [LocalTrait, TestTrait, CircularTrait, CoinductionLocalTrait],
               [NodeC, NodeB, NodeA],
               [{
                   [],
                   [(NodeA < T > : CircularTrait, T : Clone),
                   (NodeA < T > : CircularTrait, T : Send),
                   (NodeA < T > : CircularTrait, NodeA < T > : Clone),
                   (NodeA < T > : CircularTrait, typedef :: generic_types :: Container <
                   NodeB < T > , InternalType > : LocalTrait),
                   (NodeA < T > : CircularTrait, typedef :: generic_types ::
                   ConstrainedStruct < std :: iter :: Once < NodeC < T > > > :
                   CircularTrait)], [T : 'static + Clone + Send]
               }, None, None, None, None, None, None, None, None, None, None, None, None,
               None, None, None, None, None, None, None,
               {
                   [],
                   [(NodeA < T > : TestTrait, typedef :: generic_types :: Container <
                   NodeB < T > , NodeC < T > > : LocalTrait)], [T]
               }, { [NodeB < T > : LocalTrait], [], [T] },
               {
                   [],
                   [(NodeB < T > : CircularTrait, T : Clone),
                   (NodeB < T > : CircularTrait, T : Send),
                   (NodeB < T > : CircularTrait, T : Clone),
                   (NodeB < T > : CircularTrait, typedef :: generic_types :: Wrapper <
                   NodeA < T > > : LocalTrait),
                   (NodeB < T > : CircularTrait, typedef :: generic_types :: MultiGeneric
                   < NodeC < T > , NodeB < T > , String > : CircularTrait)],
                   [T : 'static + Clone + Send]
               },
               {
                   [],
                   [(NodeC < T > : CircularTrait, T : Clone),
                   (NodeC < T > : CircularTrait, T : Send),
                   (NodeC < T > : CircularTrait, T : Clone),
                   (NodeC < T > : CircularTrait, NodeA < T > : TestTrait),
                   (NodeC < T > : CircularTrait, NodeB < T > : LocalTrait),
                   (NodeC < T > : CircularTrait, typedef :: generic_types :: Container <
                   NodeA < T > , NodeB < T > > : LocalTrait),
                   (NodeA < T > : TestTrait, typedef :: generic_types :: Container <
                   NodeB < T > , NodeC < T > > : LocalTrait)],
                   [T : 'static + Clone + Send]
               },
               {
                   [],
                   [(NodeA < T > : CoinductionLocalTrait, T : Clone),
                   (NodeA < T > : CoinductionLocalTrait, typedef :: generic_types ::
                   Wrapper < NodeB < T > > : LocalTrait)], [T : Clone]
               }, { [NodeB < T > : CoinductionLocalTrait], [], [T] },
               {
                   [],
                   [(NodeC < T > : CoinductionLocalTrait, NodeA < T > : TestTrait),
                   (NodeC < T > : CoinductionLocalTrait, NodeA < T > :
                   CoinductionLocalTrait),
                   (NodeC < T > : CoinductionLocalTrait, NodeB < T > : LocalTrait),
                   (NodeC < T > : CoinductionLocalTrait, NodeB < T > :
                   CoinductionLocalTrait),
                   (NodeA < T > : TestTrait, typedef :: generic_types :: Container <
                   NodeB < T > , NodeC < T > > : LocalTrait),
                   (NodeA < T > : CoinductionLocalTrait, T : Clone),
                   (NodeA < T > : CoinductionLocalTrait, typedef :: generic_types ::
                   Wrapper < NodeB < T > > : LocalTrait)], [T]
               }],
               [impl < T : 'static + Clone + Send > CircularTrait for NodeA < T > where
               NodeA < T > : Clone, typedef :: generic_types :: Container < NodeB < T > ,
               InternalType > : LocalTrait, typedef :: generic_types :: ConstrainedStruct
               < std :: iter :: Once < NodeC < T > > > : CircularTrait,
               {
                   fn circular_method(& self) -> Box < dyn CircularTrait >
                   {
                       Box ::
                       new(NodeB :: < T >
                       {
                           count : self.value.len(), child_a : None, internal :
                           InternalType(42.0), phantom : PhantomData,
                       })
                   }
               }, impl < T : Clone > Clone for NodeA < T >
               {
                   fn clone(& self) -> Self
                   {
                       NodeA
                       {
                           value : self.value.clone(), child_b : self.child_b.clone(),
                           phantom : self.phantom,
                       }
                   }
               }, impl < T > std :: fmt :: Debug for NodeA < T >
               {
                   fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt
                   :: Result
                   {
                       f.debug_struct("NodeA").field("value", &
                       self.value).field("child_b", & "<child_b>").finish()
                   }
               }, impl < T > PartialEq for NodeA < T >
               { fn eq(& self, other : & Self) -> bool { self.value == other.value } },
               impl < T > ToString for NodeA < T >
               { fn to_string(& self) -> String { format! ("NodeA({})", self.value) } },
               unsafe impl < T > Send for NodeA < T > {}, unsafe impl < T > Sync for
               NodeA < T > {}, impl < T : Clone > Clone for NodeB < T >
               {
                   fn clone(& self) -> Self
                   {
                       NodeB
                       {
                           count : self.count, child_a : self.child_a.clone(), internal :
                           self.internal.clone(), phantom : self.phantom,
                       }
                   }
               }, impl < T > std :: fmt :: Debug for NodeB < T >
               {
                   fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt
                   :: Result
                   {
                       f.debug_struct("NodeB").field("count", &
                       self.count).field("child_a", &
                       "<child_a>").field("internal", & self.internal).finish()
                   }
               }, impl < T > PartialEq for NodeB < T >
               {
                   fn eq(& self, other : & Self) -> bool
                   { self.count == other.count && self.internal == other.internal }
               }, unsafe impl < T > Send for NodeB < T > {}, unsafe impl < T > Sync for
               NodeB < T > {}, impl < T : Clone > Clone for NodeC < T >
               {
                   fn clone(& self) -> Self
                   {
                       NodeC
                       {
                           data : self.data, ref_a : self.ref_a.clone(), ref_b :
                           self.ref_b.clone(), phantom : self.phantom,
                       }
                   }
               }, impl < T > std :: fmt :: Debug for NodeC < T >
               {
                   fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt
                   :: Result
                   {
                       f.debug_struct("NodeC").field("data", &
                       self.data).field("ref_a", &
                       "<ref_a>").field("ref_b", & "<ref_b>").finish()
                   }
               }, impl < T > PartialEq for NodeC < T >
               { fn eq(& self, other : & Self) -> bool { self.data == other.data } },
               impl < T > std :: hash :: Hash for NodeC < T >
               {
                   fn hash < H : std :: hash :: Hasher > (& self, state : & mut H)
                   { self.data.hash(state); }
               }, impl < T > std :: hash :: Hash for NodeB < T >
               {
                   fn hash < H : std :: hash :: Hasher > (& self, state : & mut H)
                   { self.count.hash(state); }
               }, impl < T > Default for NodeA < T >
               {
                   fn default() -> Self
                   {
                       NodeA
                       {
                           value : String :: new(), child_b : None, phantom :
                           PhantomData,
                       }
                   }
               }, impl < T > Default for NodeB < T >
               {
                   fn default() -> Self
                   {
                       NodeB
                       {
                           count : 0, child_a : None, internal : InternalType(0.0),
                           phantom : PhantomData,
                       }
                   }
               }, impl < T > Default for NodeC < T >
               {
                   fn default() -> Self
                   {
                       NodeC
                       { data : 0, ref_a : None, ref_b : None, phantom : PhantomData, }
                   }
               }, impl < T > TestTrait for NodeA < T > where typedef :: generic_types ::
               Container < NodeB < T > , NodeC < T > > : LocalTrait,
               {
                   fn test_method(& self) -> String { format! ("NodeA: {}", self.value) }
               }, impl < T > LocalTrait for NodeB < T >
               {
                   fn local_method(& self) -> usize
                   { self.count + (self.internal.coinduction_method() as usize) }
               }, impl < T : 'static + Clone + Send > CircularTrait for NodeB < T > where
               T : Clone, typedef :: generic_types :: Wrapper < NodeA < T > > :
               LocalTrait, typedef :: generic_types :: MultiGeneric < NodeC < T > , NodeB
               < T > , String > : CircularTrait,
               {
                   fn circular_method(& self) -> Box < dyn CircularTrait >
                   {
                       Box ::
                       new(NodeC :: < T >
                       {
                           data : self.count as i32, ref_a : None, ref_b : None, phantom
                           : PhantomData,
                       })
                   }
               }, impl < T : 'static + Clone + Send > CircularTrait for NodeC < T > where
               T : Clone, NodeA < T > : TestTrait, NodeB < T > : LocalTrait, typedef ::
               generic_types :: Container < NodeA < T > , NodeB < T > > : LocalTrait,
               {
                   fn circular_method(& self) -> Box < dyn CircularTrait >
                   {
                       Box ::
                       new(NodeA :: < T >
                       {
                           value : format! ("Generated from NodeC: {}", self.data),
                           child_b : None, phantom : PhantomData,
                       })
                   }
               }, impl < T : Clone > CoinductionLocalTrait for NodeA < T > where typedef
               :: generic_types :: Wrapper < NodeB < T > > : LocalTrait,
               { fn coinduction_method(& self) -> f64 { self.value.len() as f64 } }, impl
               < T > CoinductionLocalTrait for NodeB < T >
               {
                   fn coinduction_method(& self) -> f64
                   { self.count as f64 * self.internal.0 }
               }, impl < T > CoinductionLocalTrait for NodeC < T > where NodeA < T > :
               TestTrait + CoinductionLocalTrait, NodeB < T > : LocalTrait +
               CoinductionLocalTrait,
               { fn coinduction_method(& self) -> f64 { self.data as f64 * 2.0 } }]
           }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__next_step"
            && e.input == r#""0.2.0", Typedef
{
    predicates :
    [([__T_12_13753066654786479059], ConstrainedStruct <
    __T_12_13753066654786479059 > : CircularTrait,
    [__T_12_13753066654786479059 : Iterator, __T_12_13753066654786479059 :
    Clone, __T_12_13753066654786479059 : Send, T :: Item : :: std :: fmt ::
    Debug])]
},
[typedef :: generic_types :: ConstrainedStruct <
std :: iter :: Once < NodeC < T > > > : CircularTrait, typedef ::
generic_types :: Container < NodeB < T > , NodeC < T > > : LocalTrait, typedef
:: generic_types :: MultiGeneric < NodeC < T > , NodeB < T > , String > :
CircularTrait, typedef :: generic_types :: Wrapper < NodeB < T > > :
LocalTrait, typedef :: generic_types :: Container < NodeB < T > , InternalType
> : LocalTrait, typedef :: generic_types :: Wrapper < NodeA < T > > :
LocalTrait, typedef :: generic_types :: Container < NodeA < T > , NodeB < T >
> : LocalTrait], { :: coinduction },
[LocalTrait, TestTrait, CircularTrait, CoinductionLocalTrait],
[NodeA, NodeB, NodeC],
[{
    [],
    [(NodeA < T > : CircularTrait, T : Clone),
    (NodeA < T > : CircularTrait, T : Send),
    (NodeA < T > : CircularTrait, NodeA < T > : Clone),
    (NodeA < T > : CircularTrait, typedef :: generic_types :: Container <
    NodeB < T > , InternalType > : LocalTrait),
    (NodeA < T > : CircularTrait, typedef :: generic_types ::
    ConstrainedStruct < std :: iter :: Once < NodeC < T > > > :
    CircularTrait)], [T : 'static + Clone + Send]
}, None, None, None, None, None, None, None, None, None, None, None, None,
None, None, None, None, None, None, None,
{
    [],
    [(NodeA < T > : TestTrait, typedef :: generic_types :: Container < NodeB <
    T > , NodeC < T > > : LocalTrait)], [T]
}, { [NodeB < T > : LocalTrait], [], [T] },
{
    [],
    [(NodeB < T > : CircularTrait, T : Clone),
    (NodeB < T > : CircularTrait, T : Send),
    (NodeB < T > : CircularTrait, T : Clone),
    (NodeB < T > : CircularTrait, typedef :: generic_types :: Wrapper < NodeA
    < T > > : LocalTrait),
    (NodeB < T > : CircularTrait, typedef :: generic_types :: MultiGeneric <
    NodeC < T > , NodeB < T > , String > : CircularTrait)],
    [T : 'static + Clone + Send]
},
{
    [],
    [(NodeC < T > : CircularTrait, T : Clone),
    (NodeC < T > : CircularTrait, T : Send),
    (NodeC < T > : CircularTrait, T : Clone),
    (NodeC < T > : CircularTrait, NodeA < T > : TestTrait),
    (NodeC < T > : CircularTrait, NodeB < T > : LocalTrait),
    (NodeC < T > : CircularTrait, typedef :: generic_types :: Container <
    NodeA < T > , NodeB < T > > : LocalTrait),
    (NodeA < T > : TestTrait, typedef :: generic_types :: Container < NodeB <
    T > , NodeC < T > > : LocalTrait)], [T : 'static + Clone + Send]
},
{
    [],
    [(NodeA < T > : CoinductionLocalTrait, T : Clone),
    (NodeA < T > : CoinductionLocalTrait, typedef :: generic_types :: Wrapper
    < NodeB < T > > : LocalTrait)], [T : Clone]
}, { [NodeB < T > : CoinductionLocalTrait], [], [T] },
{
    [],
    [(NodeC < T > : CoinductionLocalTrait, NodeA < T > : TestTrait),
    (NodeC < T > : CoinductionLocalTrait, NodeA < T > :
    CoinductionLocalTrait),
    (NodeC < T > : CoinductionLocalTrait, NodeB < T > : LocalTrait),
    (NodeC < T > : CoinductionLocalTrait, NodeB < T > :
    CoinductionLocalTrait),
    (NodeA < T > : TestTrait, typedef :: generic_types :: Container < NodeB <
    T > , NodeC < T > > : LocalTrait),
    (NodeA < T > : CoinductionLocalTrait, T : Clone),
    (NodeA < T > : CoinductionLocalTrait, typedef :: generic_types :: Wrapper
    < NodeB < T > > : LocalTrait)], [T]
}],
[impl < T : 'static + Clone + Send > CircularTrait for NodeA < T > where NodeA
< T > : Clone, typedef :: generic_types :: Container < NodeB < T > ,
InternalType > : LocalTrait, typedef :: generic_types :: ConstrainedStruct <
std :: iter :: Once < NodeC < T > > > : CircularTrait,
{
    fn circular_method(& self) -> Box < dyn CircularTrait >
    {
        Box ::
        new(NodeB :: < T >
        {
            count : self.value.len(), child_a : None, internal :
            InternalType(42.0), phantom : PhantomData,
        })
    }
}, impl < T : Clone > Clone for NodeA < T >
{
    fn clone(& self) -> Self
    {
        NodeA
        {
            value : self.value.clone(), child_b : self.child_b.clone(),
            phantom : self.phantom,
        }
    }
}, impl < T > std :: fmt :: Debug for NodeA < T >
{
    fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt ::
    Result
    {
        f.debug_struct("NodeA").field("value", &
        self.value).field("child_b", & "<child_b>").finish()
    }
}, impl < T > PartialEq for NodeA < T >
{ fn eq(& self, other : & Self) -> bool { self.value == other.value } }, impl
< T > ToString for NodeA < T >
{ fn to_string(& self) -> String { format! ("NodeA({})", self.value) } },
unsafe impl < T > Send for NodeA < T > {}, unsafe impl < T > Sync for NodeA <
T > {}, impl < T : Clone > Clone for NodeB < T >
{
    fn clone(& self) -> Self
    {
        NodeB
        {
            count : self.count, child_a : self.child_a.clone(), internal :
            self.internal.clone(), phantom : self.phantom,
        }
    }
}, impl < T > std :: fmt :: Debug for NodeB < T >
{
    fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt ::
    Result
    {
        f.debug_struct("NodeB").field("count", &
        self.count).field("child_a", &
        "<child_a>").field("internal", & self.internal).finish()
    }
}, impl < T > PartialEq for NodeB < T >
{
    fn eq(& self, other : & Self) -> bool
    { self.count == other.count && self.internal == other.internal }
}, unsafe impl < T > Send for NodeB < T > {}, unsafe impl < T > Sync for NodeB
< T > {}, impl < T : Clone > Clone for NodeC < T >
{
    fn clone(& self) -> Self
    {
        NodeC
        {
            data : self.data, ref_a : self.ref_a.clone(), ref_b :
            self.ref_b.clone(), phantom : self.phantom,
        }
    }
}, impl < T > std :: fmt :: Debug for NodeC < T >
{
    fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt ::
    Result
    {
        f.debug_struct("NodeC").field("data", &
        self.data).field("ref_a", &
        "<ref_a>").field("ref_b", & "<ref_b>").finish()
    }
}, impl < T > PartialEq for NodeC < T >
{ fn eq(& self, other : & Self) -> bool { self.data == other.data } }, impl <
T > std :: hash :: Hash for NodeC < T >
{
    fn hash < H : std :: hash :: Hasher > (& self, state : & mut H)
    { self.data.hash(state); }
}, impl < T > std :: hash :: Hash for NodeB < T >
{
    fn hash < H : std :: hash :: Hasher > (& self, state : & mut H)
    { self.count.hash(state); }
}, impl < T > Default for NodeA < T >
{
    fn default() -> Self
    {
        NodeA
        { value : String :: new(), child_b : None, phantom : PhantomData, }
    }
}, impl < T > Default for NodeB < T >
{
    fn default() -> Self
    {
        NodeB
        {
            count : 0, child_a : None, internal : InternalType(0.0), phantom :
            PhantomData,
        }
    }
}, impl < T > Default for NodeC < T >
{
    fn default() -> Self
    { NodeC { data : 0, ref_a : None, ref_b : None, phantom : PhantomData, } }
}, impl < T > TestTrait for NodeA < T > where typedef :: generic_types ::
Container < NodeB < T > , NodeC < T > > : LocalTrait,
{ fn test_method(& self) -> String { format! ("NodeA: {}", self.value) } },
impl < T > LocalTrait for NodeB < T >
{
    fn local_method(& self) -> usize
    { self.count + (self.internal.coinduction_method() as usize) }
}, impl < T : 'static + Clone + Send > CircularTrait for NodeB < T > where T :
Clone, typedef :: generic_types :: Wrapper < NodeA < T > > : LocalTrait,
typedef :: generic_types :: MultiGeneric < NodeC < T > , NodeB < T > , String
> : CircularTrait,
{
    fn circular_method(& self) -> Box < dyn CircularTrait >
    {
        Box ::
        new(NodeC :: < T >
        {
            data : self.count as i32, ref_a : None, ref_b : None, phantom :
            PhantomData,
        })
    }
}, impl < T : 'static + Clone + Send > CircularTrait for NodeC < T > where T :
Clone, NodeA < T > : TestTrait, NodeB < T > : LocalTrait, typedef ::
generic_types :: Container < NodeA < T > , NodeB < T > > : LocalTrait,
{
    fn circular_method(& self) -> Box < dyn CircularTrait >
    {
        Box ::
        new(NodeA :: < T >
        {
            value : format! ("Generated from NodeC: {}", self.data), child_b :
            None, phantom : PhantomData,
        })
    }
}, impl < T : Clone > CoinductionLocalTrait for NodeA < T > where typedef ::
generic_types :: Wrapper < NodeB < T > > : LocalTrait,
{ fn coinduction_method(& self) -> f64 { self.value.len() as f64 } }, impl < T
> CoinductionLocalTrait for NodeB < T >
{
    fn coinduction_method(& self) -> f64
    { self.count as f64 * self.internal.0 }
}, impl < T > CoinductionLocalTrait for NodeC < T > where NodeA < T > :
TestTrait + CoinductionLocalTrait, NodeB < T > : LocalTrait +
CoinductionLocalTrait,
{ fn coinduction_method(& self) -> f64 { self.data as f64 * 2.0 } }]"#
            && e.to == r#"LocalTrait!
{
    "0.2.0", None,
    [typedef :: generic_types :: Container < NodeB < T > , NodeC < T > > :
    LocalTrait, typedef :: generic_types :: MultiGeneric < NodeC < T > , NodeB
    < T > , String > : CircularTrait, typedef :: generic_types :: Wrapper <
    NodeB < T > > : LocalTrait, typedef :: generic_types :: Container < NodeB
    < T > , InternalType > : LocalTrait, typedef :: generic_types :: Wrapper <
    NodeA < T > > : LocalTrait, typedef :: generic_types :: Container < NodeA
    < T > , NodeB < T > > : LocalTrait], { :: coinduction },
    [LocalTrait, TestTrait, CircularTrait, CoinductionLocalTrait],
    [NodeC, NodeA, NodeB],
    [{
        [],
        [(NodeA < T > : CircularTrait, T : Clone),
        (NodeA < T > : CircularTrait, T : Send),
        (NodeA < T > : CircularTrait, NodeA < T > : Clone),
        (NodeA < T > : CircularTrait, typedef :: generic_types :: Container <
        NodeB < T > , InternalType > : LocalTrait),
        (NodeA < T > : CircularTrait, typedef :: generic_types ::
        ConstrainedStruct < std :: iter :: Once < NodeC < T > > > :
        CircularTrait)], [T : 'static + Clone + Send]
    }, None, None, None, None, None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None,
    {
        [],
        [(NodeA < T > : TestTrait, typedef :: generic_types :: Container <
        NodeB < T > , NodeC < T > > : LocalTrait)], [T]
    }, { [NodeB < T > : LocalTrait], [], [T] },
    {
        [],
        [(NodeB < T > : CircularTrait, T : Clone),
        (NodeB < T > : CircularTrait, T : Send),
        (NodeB < T > : CircularTrait, T : Clone),
        (NodeB < T > : CircularTrait, typedef :: generic_types :: Wrapper <
        NodeA < T > > : LocalTrait),
        (NodeB < T > : CircularTrait, typedef :: generic_types :: MultiGeneric
        < NodeC < T > , NodeB < T > , String > : CircularTrait)],
        [T : 'static + Clone + Send]
    },
    {
        [],
        [(NodeC < T > : CircularTrait, T : Clone),
        (NodeC < T > : CircularTrait, T : Send),
        (NodeC < T > : CircularTrait, T : Clone),
        (NodeC < T > : CircularTrait, NodeA < T > : TestTrait),
        (NodeC < T > : CircularTrait, NodeB < T > : LocalTrait),
        (NodeC < T > : CircularTrait, typedef :: generic_types :: Container <
        NodeA < T > , NodeB < T > > : LocalTrait),
        (NodeA < T > : TestTrait, typedef :: generic_types :: Container <
        NodeB < T > , NodeC < T > > : LocalTrait)],
        [T : 'static + Clone + Send]
    },
    {
        [],
        [(NodeA < T > : CoinductionLocalTrait, T : Clone),
        (NodeA < T > : CoinductionLocalTrait, typedef :: generic_types ::
        Wrapper < NodeB < T > > : LocalTrait)], [T : Clone]
    }, { [NodeB < T > : CoinductionLocalTrait], [], [T] },
    {
        [],
        [(NodeC < T > : CoinductionLocalTrait, NodeA < T > : TestTrait),
        (NodeC < T > : CoinductionLocalTrait, NodeA < T > :
        CoinductionLocalTrait),
        (NodeC < T > : CoinductionLocalTrait, NodeB < T > : LocalTrait),
        (NodeC < T > : CoinductionLocalTrait, NodeB < T > :
        CoinductionLocalTrait),
        (NodeA < T > : TestTrait, typedef :: generic_types :: Container <
        NodeB < T > , NodeC < T > > : LocalTrait),
        (NodeA < T > : CoinductionLocalTrait, T : Clone),
        (NodeA < T > : CoinductionLocalTrait, typedef :: generic_types ::
        Wrapper < NodeB < T > > : LocalTrait)], [T]
    }],
    [impl < T : 'static + Clone + Send > CircularTrait for NodeA < T > where
    NodeA < T > : Clone, typedef :: generic_types :: Container < NodeB < T > ,
    InternalType > : LocalTrait, typedef :: generic_types :: ConstrainedStruct
    < std :: iter :: Once < NodeC < T > > > : CircularTrait,
    {
        fn circular_method(& self) -> Box < dyn CircularTrait >
        {
            Box ::
            new(NodeB :: < T >
            {
                count : self.value.len(), child_a : None, internal :
                InternalType(42.0), phantom : PhantomData,
            })
        }
    }, impl < T : Clone > Clone for NodeA < T >
    {
        fn clone(& self) -> Self
        {
            NodeA
            {
                value : self.value.clone(), child_b : self.child_b.clone(),
                phantom : self.phantom,
            }
        }
    }, impl < T > std :: fmt :: Debug for NodeA < T >
    {
        fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt
        :: Result
        {
            f.debug_struct("NodeA").field("value", &
            self.value).field("child_b", & "<child_b>").finish()
        }
    }, impl < T > PartialEq for NodeA < T >
    { fn eq(& self, other : & Self) -> bool { self.value == other.value } },
    impl < T > ToString for NodeA < T >
    { fn to_string(& self) -> String { format! ("NodeA({})", self.value) } },
    unsafe impl < T > Send for NodeA < T > {}, unsafe impl < T > Sync for
    NodeA < T > {}, impl < T : Clone > Clone for NodeB < T >
    {
        fn clone(& self) -> Self
        {
            NodeB
            {
                count : self.count, child_a : self.child_a.clone(), internal :
                self.internal.clone(), phantom : self.phantom,
            }
        }
    }, impl < T > std :: fmt :: Debug for NodeB < T >
    {
        fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt
        :: Result
        {
            f.debug_struct("NodeB").field("count", &
            self.count).field("child_a", &
            "<child_a>").field("internal", & self.internal).finish()
        }
    }, impl < T > PartialEq for NodeB < T >
    {
        fn eq(& self, other : & Self) -> bool
        { self.count == other.count && self.internal == other.internal }
    }, unsafe impl < T > Send for NodeB < T > {}, unsafe impl < T > Sync for
    NodeB < T > {}, impl < T : Clone > Clone for NodeC < T >
    {
        fn clone(& self) -> Self
        {
            NodeC
            {
                data : self.data, ref_a : self.ref_a.clone(), ref_b :
                self.ref_b.clone(), phantom : self.phantom,
            }
        }
    }, impl < T > std :: fmt :: Debug for NodeC < T >
    {
        fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt
        :: Result
        {
            f.debug_struct("NodeC").field("data", &
            self.data).field("ref_a", &
            "<ref_a>").field("ref_b", & "<ref_b>").finish()
        }
    }, impl < T > PartialEq for NodeC < T >
    { fn eq(& self, other : & Self) -> bool { self.data == other.data } },
    impl < T > std :: hash :: Hash for NodeC < T >
    {
        fn hash < H : std :: hash :: Hasher > (& self, state : & mut H)
        { self.data.hash(state); }
    }, impl < T > std :: hash :: Hash for NodeB < T >
    {
        fn hash < H : std :: hash :: Hasher > (& self, state : & mut H)
        { self.count.hash(state); }
    }, impl < T > Default for NodeA < T >
    {
        fn default() -> Self
        {
            NodeA
            {
                value : String :: new(), child_b : None, phantom :
                PhantomData,
            }
        }
    }, impl < T > Default for NodeB < T >
    {
        fn default() -> Self
        {
            NodeB
            {
                count : 0, child_a : None, internal : InternalType(0.0),
                phantom : PhantomData,
            }
        }
    }, impl < T > Default for NodeC < T >
    {
        fn default() -> Self
        {
            NodeC
            { data : 0, ref_a : None, ref_b : None, phantom : PhantomData, }
        }
    }, impl < T > TestTrait for NodeA < T > where typedef :: generic_types ::
    Container < NodeB < T > , NodeC < T > > : LocalTrait,
    {
        fn test_method(& self) -> String { format! ("NodeA: {}", self.value) }
    }, impl < T > LocalTrait for NodeB < T >
    {
        fn local_method(& self) -> usize
        { self.count + (self.internal.coinduction_method() as usize) }
    }, impl < T : 'static + Clone + Send > CircularTrait for NodeB < T > where
    T : Clone, typedef :: generic_types :: Wrapper < NodeA < T > > :
    LocalTrait, typedef :: generic_types :: MultiGeneric < NodeC < T > , NodeB
    < T > , String > : CircularTrait,
    {
        fn circular_method(& self) -> Box < dyn CircularTrait >
        {
            Box ::
            new(NodeC :: < T >
            {
                data : self.count as i32, ref_a : None, ref_b : None, phantom
                : PhantomData,
            })
        }
    }, impl < T : 'static + Clone + Send > CircularTrait for NodeC < T > where
    T : Clone, NodeA < T > : TestTrait, NodeB < T > : LocalTrait, typedef ::
    generic_types :: Container < NodeA < T > , NodeB < T > > : LocalTrait,
    {
        fn circular_method(& self) -> Box < dyn CircularTrait >
        {
            Box ::
            new(NodeA :: < T >
            {
                value : format! ("Generated from NodeC: {}", self.data),
                child_b : None, phantom : PhantomData,
            })
        }
    }, impl < T : Clone > CoinductionLocalTrait for NodeA < T > where typedef
    :: generic_types :: Wrapper < NodeB < T > > : LocalTrait,
    { fn coinduction_method(& self) -> f64 { self.value.len() as f64 } }, impl
    < T > CoinductionLocalTrait for NodeB < T >
    {
        fn coinduction_method(& self) -> f64
        { self.count as f64 * self.internal.0 }
    }, impl < T > CoinductionLocalTrait for NodeC < T > where NodeA < T > :
    TestTrait + CoinductionLocalTrait, NodeB < T > : LocalTrait +
    CoinductionLocalTrait,
    { fn coinduction_method(& self) -> f64 { self.data as f64 * 2.0 } }]
}"#
    }));
}

#[test]
fn external_crate_coinduction_test_complex() {
    let expansions = run_trace_for_repo("coinduction", Some("complex"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "coinduction"
            && e.input == r#"mod coinduction_mod
{
    use super::*; pub struct
    RecA<T>(pub Option<RecB<T>>, pub core::marker::PhantomData<T>); impl<S, T>
    TraitA<S> for RecA<T> where RecB<T>: TraitB<S>, T: UpperHex +
    std::default::Default,
    {
        fn get_a(&self) -> String
        {
            if let Some(b) = &self.0
            {
                format!("{:X} {}", T::default(), <RecB<T> as
                TraitB<S>>::get_b(b))
            } else { format!("None") }
        }
    } pub struct
    RecB<T>(pub Option<Box<RecA<T>>>, pub core::marker::PhantomData<T>);
    impl<S, T> TraitB<S> for RecB<T> where RecA<T>: TraitA<S>, T: Display +
    std::default::Default,
    {
        fn get_b(&self) -> String
        {
            if let Some(a) = &self.0
            {
                format!("{} {}", T::default(), <RecA<T> as
                TraitA<S>>::get_a(a.as_ref()))
            } else { format!("None") }
        }
    }
}"#
            && e.to == r#"mod coinduction_mod
{
    use super :: * ; pub struct RecA < T >
    (pub Option < RecB < T > > , pub core :: marker :: PhantomData < T >); pub
    struct RecB < T >
    (pub Option < Box < RecA < T > > > , pub core :: marker :: PhantomData < T
    >); impl < S, T > TraitA < S > for RecA < T > where T : UpperHex, T : std
    :: default :: Default, T : Display, T : UpperHex + std :: default ::
    Default,
    {
        fn get_a(& self) -> String
        {
            if let Some(b) = & self.0
            {
                format!
                ("{:X} {}", T::default(), <RecB<T> as TraitB<S>>::get_b(b))
            } else { format! ("None") }
        }
    } impl < S, T > TraitB < S > for RecB < T > where T : UpperHex, T : std ::
    default :: Default, T : Display, T : Display + std :: default :: Default,
    {
        fn get_b(& self) -> String
        {
            if let Some(a) = & self.0
            {
                format!
                ("{} {}", T::default(), <RecA<T> as
                TraitA<S>>::get_a(a.as_ref()))
            } else { format! ("None") }
        }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "traitdef"
            && e.input == r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#
            && e.to == r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_3044150873991545574
{
    ("0.2.0", None, [[$T:ty; $N:expr] :$ ($wt : tt) *],
    { $ ($coinduction : tt) + }, $ ($t : tt) *) =>
    {
        $ ($coinduction) +:: __next_step!
        {
            "0.2.0", Traitdef
            {
                appending_constraints :
                [$T : :: core :: clone :: Clone + :: core :: default ::
                Default]
            }, [[$T; $N] :$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
        }
    };
    ("0.2.0", None, [[$T:ty] :$ ($wt : tt) *], { $ ($coinduction : tt) + }, $
    ($t : tt) *) =>
    {
        $ ($coinduction) +:: __next_step!
        {
            "0.2.0", Traitdef
            {
                appending_constraints :
                [$T : :: core :: cmp :: PartialEq + :: core :: clone :: Clone]
            }, [[$T] :$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
        }
    };
    ("0.2.0", None, [($T:ty, $U:ty) :$ ($wt : tt) *],
    { $ ($coinduction : tt) + }, $ ($t : tt) *) =>
    {
        $ ($coinduction) +:: __next_step!
        {
            "0.2.0", Traitdef
            {
                appending_constraints :
                [$T : :: core :: clone :: Clone + :: core :: marker :: Send,
                $U : :: core :: marker :: Sync + :: core :: default ::
                Default]
            }, [($T, $U) :$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
        }
    };
    ("0.2.0", None, [($T:ty, $U:ty, $V:ty) :$ ($wt : tt) *],
    { $ ($coinduction : tt) + }, $ ($t : tt) *) =>
    {
        $ ($coinduction) +:: __next_step!
        {
            "0.2.0", Traitdef
            {
                appending_constraints :
                [$T : :: core :: clone :: Clone + :: core :: marker :: Send,
                $U : :: core :: marker :: Sync, $V : :: core :: default ::
                Default + :: core :: marker :: Send]
            }, [($T, $U, $V) :$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
        }
    };
    ("0.2.0", None,
    [:: $seg0 : ident $ (:: $segs : ident) * $ (<$ ($arg : ty),*$ (,) ?>) ? :$
    ($wt : tt) *], { $ ($coinduction : tt) + }, $ ($t : tt) *) =>
    {
        :: $seg0 $ (:: $segs) * !
        {
            "0.2.0", None,
            [$ty0 : :: $seg0 $ (:: $segs) * $ (<$ ($arg),*>) ? :$ ($wt) *],
            { $ ($coinduction) + }, $ ($t) *
        }
    };
    ("0.2.0", None,
    [$seg0 : ident $ (:: $segs : ident) * $ (<$ ($arg : ty),*$ (,) ?>) ? :$
    ($wt : tt) *], { $ ($coinduction : tt) + }, $ ($t : tt) *) =>
    {
        $seg0 $ (:: $segs) *!
        {
            "0.2.0", None,
            [$seg0 $ (:: $segs) * $ (<$ ($arg),*>) ? :$ ($wt) *],
            { $ ($coinduction) + }, $ ($t) *
        }
    };
} #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub use
__LocalTrait_temporal_3044150873991545574 as LocalTrait;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "typedef"
            && e.input == r#"pub mod generic_types
{
    use super::*; use std::fmt::Debug; use std::hash::Hash; pub struct
    Container<T, U> { pub first: T, pub second: U, } pub struct Wrapper<T>
    where T: Clone + Debug, { pub value: T, pub count: usize, } pub struct
    MultiGeneric<T, U, V> where T: Clone, U: Send + Sync, V: Debug + Hash,
    { pub primary: T, pub secondary: U, pub metadata: V, } pub struct
    ConstrainedStruct<T> where T: Iterator + Clone, { pub iterator: T, }
    impl<T, U> TestTrait for Container<T, U> where T: Clone +
    ::std::fmt::Debug + Send, U: ::std::fmt::Debug + Default + Sync,
    { fn test_method(&self) -> String { format!("{:?}", self.first) } }
    impl<T> TestTrait for Wrapper<T> where T: Clone + Debug + ToString,
    {
        fn test_method(&self) -> String
        { format!("{}: {}", self.value.to_string(), self.count) }
    } impl<T, U> LocalTrait for Container<T, U> where T: Clone + Send + Sync,
    U: ::std::fmt::Debug + Hash,
    { fn local_method(&self) -> usize { let _ = self.first.clone(); 42 } }
    impl<T> LocalTrait for Wrapper<T> where T: Clone + ::std::fmt::Debug +
    Default,
    { fn local_method(&self) -> usize { let _ = T::default(); self.count } }
    impl<T, U, V> CircularTrait for MultiGeneric<T, U, V> where T: Clone +
    ::std::fmt::Debug + Send + 'static, U: Send + Sync + Default, V:
    ::std::fmt::Debug + Hash + Clone,
    {
        fn circular_method(&self) -> Box<dyn CircularTrait>
        {
            Box::new(ConstrainedStruct
            { iterator: std::iter::once(self.primary.clone()), })
        }
    } impl<T> CircularTrait for ConstrainedStruct<T> where T: Iterator + Clone
    + Send, T::Item: ::std::fmt::Debug,
    {
        fn circular_method(&self) -> Box<dyn CircularTrait>
        {
            Box::new(MultiGeneric
            {
                primary: "circular".to_string(), secondary: 42u32, metadata:
                123usize,
            })
        }
    } impl<T, U> ExtendedTrait for Container<T, U> where T: PartialEq + Clone,
    U: Default + Send,
    {
        fn extended_method(&self) -> bool
        { let _default_u = U::default(); true }
    } impl<T, U, V> ExtendedTrait for MultiGeneric<T, U, V> where T: Clone +
    PartialOrd, U: Send + Sync + Clone, V: ::std::fmt::Debug + Hash + Default,
    {
        fn extended_method(&self) -> bool
        { let _ = V::default(); let _ = self.secondary.clone(); true }
    }
}"#
            && e.to == r#"pub mod generic_types
{
    use super :: * ; use std :: fmt :: Debug; use std :: hash :: Hash; pub
    struct Container < T, U > { pub first : T, pub second : U, } pub struct
    Wrapper < T > where T : Clone + Debug,
    { pub value : T, pub count : usize, } pub struct MultiGeneric < T, U, V >
    where T : Clone, U : Send + Sync, V : Debug + Hash,
    { pub primary : T, pub secondary : U, pub metadata : V, } pub struct
    ConstrainedStruct < T > where T : Iterator + Clone, { pub iterator : T, }
    impl < T, U > TestTrait for Container < T, U > where T : Clone + :: std ::
    fmt :: Debug + Send, U : :: std :: fmt :: Debug + Default + Sync,
    { fn test_method(& self) -> String { format! ("{:?}", self.first) } } impl
    < T > TestTrait for Wrapper < T > where T : Clone + Debug + ToString,
    {
        fn test_method(& self) -> String
        { format! ("{}: {}", self.value.to_string(), self.count) }
    } impl < T, U > LocalTrait for Container < T, U > where T : Clone + Send +
    Sync, U : :: std :: fmt :: Debug + Hash,
    { fn local_method(& self) -> usize { let _ = self.first.clone(); 42 } }
    impl < T > LocalTrait for Wrapper < T > where T : Clone + :: std :: fmt ::
    Debug + Default,
    {
        fn local_method(& self) -> usize
        { let _ = T :: default(); self.count }
    } impl < T, U, V > CircularTrait for MultiGeneric < T, U, V > where T :
    Clone + :: std :: fmt :: Debug + Send + 'static, U : Send + Sync +
    Default, V : :: std :: fmt :: Debug + Hash + Clone,
    {
        fn circular_method(& self) -> Box < dyn CircularTrait >
        {
            Box ::
            new(ConstrainedStruct
            { iterator : std :: iter :: once(self.primary.clone()), })
        }
    } impl < T > CircularTrait for ConstrainedStruct < T > where T : Iterator
    + Clone + Send, T :: Item : :: std :: fmt :: Debug,
    {
        fn circular_method(& self) -> Box < dyn CircularTrait >
        {
            Box ::
            new(MultiGeneric
            {
                primary : "circular".to_string(), secondary : 42u32, metadata
                : 123usize,
            })
        }
    } impl < T, U > ExtendedTrait for Container < T, U > where T : PartialEq +
    Clone, U : Default + Send,
    {
        fn extended_method(& self) -> bool
        { let _default_u = U :: default(); true }
    } impl < T, U, V > ExtendedTrait for MultiGeneric < T, U, V > where T :
    Clone + PartialOrd, U : Send + Sync + Clone, V : :: std :: fmt :: Debug +
    Hash + Default,
    {
        fn extended_method(& self) -> bool
        { let _ = V :: default(); let _ = self.secondary.clone(); true }
    }
    #[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
    #[doc(hidden)] #[macro_export] macro_rules!
    __Container_temporal_13753066654786479059
    {
        ("0.2.0", None, [$ ($wt : tt) *], { $ ($coinduction : tt) + }, $
        ($t : tt) *) =>
        {
            $ ($coinduction) +:: __next_step!
            {
                "0.2.0", Typedef
                {
                    predicates :
                    [([__T_7_13753066654786479059, __U_7_13753066654786479059],
                    Container < __T_7_13753066654786479059,
                    __U_7_13753066654786479059 > : TestTrait,
                    [__T_7_13753066654786479059 : Clone,
                    __T_7_13753066654786479059 : :: std :: fmt :: Debug,
                    __T_7_13753066654786479059 : Send,
                    __U_7_13753066654786479059 : :: std :: fmt :: Debug,
                    __U_7_13753066654786479059 : Default,
                    __U_7_13753066654786479059 : Sync]),
                    ([__T_9_13753066654786479059, __U_9_13753066654786479059],
                    Container < __T_9_13753066654786479059,
                    __U_9_13753066654786479059 > : LocalTrait,
                    [__T_9_13753066654786479059 : Clone,
                    __T_9_13753066654786479059 : Send,
                    __T_9_13753066654786479059 : Sync,
                    __U_9_13753066654786479059 : :: std :: fmt :: Debug,
                    __U_9_13753066654786479059 : Hash]),
                    ([__T_13_13753066654786479059, __U_13_13753066654786479059],
                    Container < __T_13_13753066654786479059,
                    __U_13_13753066654786479059 > : ExtendedTrait,
                    [__T_13_13753066654786479059 : PartialEq,
                    __T_13_13753066654786479059 : Clone,
                    __U_13_13753066654786479059 : Default,
                    __U_13_13753066654786479059 : Send])]
                }, [$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
            }
        }
    } #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub
    use __Container_temporal_13753066654786479059 as Container;
    #[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
    #[doc(hidden)] #[macro_export] macro_rules!
    __Wrapper_temporal_13753066654786479059
    {
        ("0.2.0", None, [$ ($wt : tt) *], { $ ($coinduction : tt) + }, $
        ($t : tt) *) =>
        {
            $ ($coinduction) +:: __next_step!
            {
                "0.2.0", Typedef
                {
                    predicates :
                    [([__T_8_13753066654786479059], Wrapper <
                    __T_8_13753066654786479059 > : TestTrait,
                    [__T_8_13753066654786479059 : Clone,
                    __T_8_13753066654786479059 : Debug,
                    __T_8_13753066654786479059 : ToString]),
                    ([__T_10_13753066654786479059], Wrapper <
                    __T_10_13753066654786479059 > : LocalTrait,
                    [__T_10_13753066654786479059 : Clone,
                    __T_10_13753066654786479059 : :: std :: fmt :: Debug,
                    __T_10_13753066654786479059 : Default])]
                }, [$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
            }
        }
    } #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub
    use __Wrapper_temporal_13753066654786479059 as Wrapper;
    #[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
    #[doc(hidden)] #[macro_export] macro_rules!
    __MultiGeneric_temporal_13753066654786479059
    {
        ("0.2.0", None, [$ ($wt : tt) *], { $ ($coinduction : tt) + }, $
        ($t : tt) *) =>
        {
            $ ($coinduction) +:: __next_step!
            {
                "0.2.0", Typedef
                {
                    predicates :
                    [([__T_11_13753066654786479059, __U_11_13753066654786479059,
                    __V_11_13753066654786479059], MultiGeneric <
                    __T_11_13753066654786479059, __U_11_13753066654786479059,
                    __V_11_13753066654786479059 > : CircularTrait,
                    [__T_11_13753066654786479059 : Clone,
                    __T_11_13753066654786479059 : :: std :: fmt :: Debug,
                    __T_11_13753066654786479059 : Send,
                    __U_11_13753066654786479059 : Send,
                    __U_11_13753066654786479059 : Sync,
                    __U_11_13753066654786479059 : Default,
                    __V_11_13753066654786479059 : :: std :: fmt :: Debug,
                    __V_11_13753066654786479059 : Hash,
                    __V_11_13753066654786479059 : Clone]),
                    ([__T_14_13753066654786479059, __U_14_13753066654786479059,
                    __V_14_13753066654786479059], MultiGeneric <
                    __T_14_13753066654786479059, __U_14_13753066654786479059,
                    __V_14_13753066654786479059 > : ExtendedTrait,
                    [__T_14_13753066654786479059 : Clone,
                    __T_14_13753066654786479059 : PartialOrd,
                    __U_14_13753066654786479059 : Send,
                    __U_14_13753066654786479059 : Sync,
                    __U_14_13753066654786479059 : Clone,
                    __V_14_13753066654786479059 : :: std :: fmt :: Debug,
                    __V_14_13753066654786479059 : Hash,
                    __V_14_13753066654786479059 : Default])]
                }, [$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
            }
        }
    } #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub
    use __MultiGeneric_temporal_13753066654786479059 as MultiGeneric;
    #[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
    #[doc(hidden)] #[macro_export] macro_rules!
    __ConstrainedStruct_temporal_13753066654786479059
    {
        ("0.2.0", None, [$ ($wt : tt) *], { $ ($coinduction : tt) + }, $
        ($t : tt) *) =>
        {
            $ ($coinduction) +:: __next_step!
            {
                "0.2.0", Typedef
                {
                    predicates :
                    [([__T_12_13753066654786479059], ConstrainedStruct <
                    __T_12_13753066654786479059 > : CircularTrait,
                    [__T_12_13753066654786479059 : Iterator,
                    __T_12_13753066654786479059 : Clone,
                    __T_12_13753066654786479059 : Send, T :: Item : :: std ::
                    fmt :: Debug])]
                }, [$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
            }
        }
    } #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub
    use __ConstrainedStruct_temporal_13753066654786479059 as
    ConstrainedStruct;
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__TraitA_temporal_8852313351335435875"
            && e.input == r#""0.2.0", None,
            [Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 > : TraitA < S >],
            { :: coinduction }, [TraitB, TraitA], [RecC, RecD],
            [{
                [],
                [(RecC < T1, T2, T3, T4 > : TraitA < S > ,
                (T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
                < S >), (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Display),
                (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Default),
                ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
                < S > , T1 : TraitB < S >),
                ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
                < S > , Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 > :
                TraitA < S >),
                ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
                < S > , S : Display),
                ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
                < S > , S : Default)], [T1, T2, T4, T3, S]
            },
            {
                [],
                [(RecD < T1, T2, T3, T4 > : TraitB < S > , RecC < T1, T2, T3, T4 > :
                TraitA < S >),
                (RecD < T1, T2, T3, T4 > : TraitB < S > , T1 : TraitB < S >),
                (RecC < T1, T2, T3, T4 > : TraitA < S > ,
                (T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
                < S >), (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Display),
                (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Default),
                ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
                < S > , T1 : TraitB < S >),
                ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
                < S > , Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 > :
                TraitA < S >),
                ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
                < S > , S : Display),
                ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
                < S > , S : Default)], [S, T4, T2, T3, T1]
            }],
            [impl < T1, T2, T3, T4, S > TraitA < S > for RecC < T1, T2, T3, T4 > where
            (T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB < S
            > , S : Display + Default,
            {
                fn get_a(& self) -> String
                {
                    format!
                    ("RecC: {}",
                    <(T1, Wrapper2<(T2, (T3, (T3, RecD<T1, T2, T3, T4>))), T4>) as
                    TraitB<S>>::get_b(&self.0))
                }
            }, impl < T1, T2, T3, T4, S > TraitB < S > for RecD < T1, T2, T3, T4 > where
            RecC < T1, T2, T3, T4 > : TraitA < S > , T1 : TraitB < S > ,
            {
                fn get_b(& self) -> String
                {
                    if let Some(ref rec_c) = self.0
                    {
                        format!
                        ("RecD {}", <RecC<T1, T2, T3, T4> as TraitA<S>>::get_a(rec_c))
                    } else { format! ("RecD None") }
                }
            }]"#
            && e.to == r#"Wrapper2 !
            {
                "0.2.0", None,
                [Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 > : TraitA < S
                >], { :: coinduction }, [TraitB, TraitA], [RecC, RecD],
                [{
                    [],
                    [(RecC < T1, T2, T3, T4 > : TraitA < S > ,
                    (T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
                    TraitB < S >), (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Display),
                    (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Default),
                    ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
                    TraitB < S > , T1 : TraitB < S >),
                    ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
                    TraitB < S > ,
                    Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 > : TraitA <
                    S >),
                    ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
                    TraitB < S > , S : Display),
                    ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
                    TraitB < S > , S : Default)], [T1, T2, T4, T3, S]
                },
                {
                    [],
                    [(RecD < T1, T2, T3, T4 > : TraitB < S > , RecC < T1, T2, T3, T4 > :
                    TraitA < S >),
                    (RecD < T1, T2, T3, T4 > : TraitB < S > , T1 : TraitB < S >),
                    (RecC < T1, T2, T3, T4 > : TraitA < S > ,
                    (T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
                    TraitB < S >), (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Display),
                    (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Default),
                    ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
                    TraitB < S > , T1 : TraitB < S >),
                    ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
                    TraitB < S > ,
                    Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 > : TraitA <
                    S >),
                    ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
                    TraitB < S > , S : Display),
                    ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
                    TraitB < S > , S : Default)], [S, T4, T2, T3, T1]
                }],
                [impl < T1, T2, T3, T4, S > TraitA < S > for RecC < T1, T2, T3, T4 > where
                (T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
                < S > , S : Display + Default,
                {
                    fn get_a(& self) -> String
                    {
                        format!
                        ("RecC: {}",
                        <(T1, Wrapper2<(T2, (T3, (T3, RecD<T1, T2, T3, T4>))), T4>) as
                        TraitB<S>>::get_b(&self.0))
                    }
                }, impl < T1, T2, T3, T4, S > TraitB < S > for RecD < T1, T2, T3, T4 >
                where RecC < T1, T2, T3, T4 > : TraitA < S > , T1 : TraitB < S > ,
                {
                    fn get_b(& self) -> String
                    {
                        if let Some(ref rec_c) = self.0
                        {
                            format!
                            ("RecD {}", <RecC<T1, T2, T3, T4> as TraitA<S>>::get_a(rec_c))
                        } else { format! ("RecD None") }
                    }
                }]
            }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__TraitB_temporal_6419969987693879595"
            && e.input == r#""0.2.0", None,
            [(T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB < S
            >], { :: coinduction }, [TraitB, TraitA], [RecD, RecC],
            [{
                [],
                [(RecC < T1, T2, T3, T4 > : TraitA < S > ,
                (T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
                < S >), (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Display),
                (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Default)],
                [T1, T2, S, T3, T4]
            },
            {
                [],
                [(RecD < T1, T2, T3, T4 > : TraitB < S > , RecC < T1, T2, T3, T4 > :
                TraitA < S >),
                (RecD < T1, T2, T3, T4 > : TraitB < S > , T1 : TraitB < S >),
                (RecC < T1, T2, T3, T4 > : TraitA < S > ,
                (T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
                < S >), (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Display),
                (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Default)],
                [T1, T2, T3, S, T4]
            }],
            [impl < T1, T2, T3, T4, S > TraitA < S > for RecC < T1, T2, T3, T4 > where
            (T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB < S
            > , S : Display + Default,
            {
                fn get_a(& self) -> String
                {
                    format!
                    ("RecC: {}",
                    <(T1, Wrapper2<(T2, (T3, (T3, RecD<T1, T2, T3, T4>))), T4>) as
                    TraitB<S>>::get_b(&self.0))
                }
            }, impl < T1, T2, T3, T4, S > TraitB < S > for RecD < T1, T2, T3, T4 > where
            RecC < T1, T2, T3, T4 > : TraitA < S > , T1 : TraitB < S > ,
            {
                fn get_b(& self) -> String
                {
                    if let Some(ref rec_c) = self.0
                    {
                        format!
                        ("RecD {}", <RecC<T1, T2, T3, T4> as TraitA<S>>::get_a(rec_c))
                    } else { format! ("RecD None") }
                }
            }]"#
            && e.to == r#":: coinduction :: __next_step!
            {
                "0.2.0", Traitdef
                {
                    appending_constraints :
                    [T1 : TraitB < S > ,
                    Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 > : TraitA <
                    S > , S : Display + Default]
                },
                [(T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
                < S >], { :: coinduction }, [TraitB, TraitA], [RecD, RecC],
                [{
                    [],
                    [(RecC < T1, T2, T3, T4 > : TraitA < S > ,
                    (T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
                    TraitB < S >), (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Display),
                    (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Default)],
                    [T1, T2, S, T3, T4]
                },
                {
                    [],
                    [(RecD < T1, T2, T3, T4 > : TraitB < S > , RecC < T1, T2, T3, T4 > :
                    TraitA < S >),
                    (RecD < T1, T2, T3, T4 > : TraitB < S > , T1 : TraitB < S >),
                    (RecC < T1, T2, T3, T4 > : TraitA < S > ,
                    (T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
                    TraitB < S >), (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Display),
                    (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Default)],
                    [T1, T2, T3, S, T4]
                }],
                [impl < T1, T2, T3, T4, S > TraitA < S > for RecC < T1, T2, T3, T4 > where
                (T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
                < S > , S : Display + Default,
                {
                    fn get_a(& self) -> String
                    {
                        format!
                        ("RecC: {}",
                        <(T1, Wrapper2<(T2, (T3, (T3, RecD<T1, T2, T3, T4>))), T4>) as
                        TraitB<S>>::get_b(&self.0))
                    }
                }, impl < T1, T2, T3, T4, S > TraitB < S > for RecD < T1, T2, T3, T4 >
                where RecC < T1, T2, T3, T4 > : TraitA < S > , T1 : TraitB < S > ,
                {
                    fn get_b(& self) -> String
                    {
                        if let Some(ref rec_c) = self.0
                        {
                            format!
                            ("RecD {}", <RecC<T1, T2, T3, T4> as TraitA<S>>::get_a(rec_c))
                        } else { format! ("RecD None") }
                    }
                }]
            }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__Wrapper2_temporal_15971242928440160188"
            && e.input == r#""0.2.0", None,
            [Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 > : TraitA < S >],
            { :: coinduction }, [TraitB, TraitA], [RecC, RecD],
            [{
                [],
                [(RecC < T1, T2, T3, T4 > : TraitA < S > ,
                (T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
                < S >), (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Display),
                (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Default),
                ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
                < S > , T1 : TraitB < S >),
                ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
                < S > , Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 > :
                TraitA < S >),
                ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
                < S > , S : Display),
                ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
                < S > , S : Default)], [T1, T2, T4, T3, S]
            },
            {
                [],
                [(RecD < T1, T2, T3, T4 > : TraitB < S > , RecC < T1, T2, T3, T4 > :
                TraitA < S >),
                (RecD < T1, T2, T3, T4 > : TraitB < S > , T1 : TraitB < S >),
                (RecC < T1, T2, T3, T4 > : TraitA < S > ,
                (T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
                < S >), (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Display),
                (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Default),
                ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
                < S > , T1 : TraitB < S >),
                ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
                < S > , Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 > :
                TraitA < S >),
                ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
                < S > , S : Display),
                ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
                < S > , S : Default)], [S, T4, T2, T3, T1]
            }],
            [impl < T1, T2, T3, T4, S > TraitA < S > for RecC < T1, T2, T3, T4 > where
            (T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB < S
            > , S : Display + Default,
            {
                fn get_a(& self) -> String
                {
                    format!
                    ("RecC: {}",
                    <(T1, Wrapper2<(T2, (T3, (T3, RecD<T1, T2, T3, T4>))), T4>) as
                    TraitB<S>>::get_b(&self.0))
                }
            }, impl < T1, T2, T3, T4, S > TraitB < S > for RecD < T1, T2, T3, T4 > where
            RecC < T1, T2, T3, T4 > : TraitA < S > , T1 : TraitB < S > ,
            {
                fn get_b(& self) -> String
                {
                    if let Some(ref rec_c) = self.0
                    {
                        format!
                        ("RecD {}", <RecC<T1, T2, T3, T4> as TraitA<S>>::get_a(rec_c))
                    } else { format! ("RecD None") }
                }
            }]"#
            && e.to == r#":: coinduction :: __next_step!
            {
                "0.2.0", Typedef
                {
                    predicates :
                    [([__T_9_15971242928440160188, __S_9_15971242928440160188,
                    __U_9_15971242928440160188], Wrapper2 < __T_9_15971242928440160188,
                    __U_9_15971242928440160188 > : TraitA < __S_9_15971242928440160188 > ,
                    [__T_9_15971242928440160188 : TraitA < __S_9_15971242928440160188 > ,
                    __U_9_15971242928440160188 : Default, __U_9_15971242928440160188 :
                    Display]),
                    ([__T_10_15971242928440160188, __S_10_15971242928440160188,
                    __U_10_15971242928440160188], Wrapper2 < __T_10_15971242928440160188,
                    __U_10_15971242928440160188 > : TraitB < __S_10_15971242928440160188 >
                    ,
                    [__T_10_15971242928440160188 : TraitB < __S_10_15971242928440160188 >
                    , __U_10_15971242928440160188 : Default, __U_10_15971242928440160188 :
                    Display])]
                },
                [Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 > : TraitA < S
                >], { :: coinduction }, [TraitB, TraitA], [RecC, RecD],
                [{
                    [],
                    [(RecC < T1, T2, T3, T4 > : TraitA < S > ,
                    (T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
                    TraitB < S >), (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Display),
                    (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Default),
                    ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
                    TraitB < S > , T1 : TraitB < S >),
                    ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
                    TraitB < S > ,
                    Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 > : TraitA <
                    S >),
                    ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
                    TraitB < S > , S : Display),
                    ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
                    TraitB < S > , S : Default)], [T1, T2, T4, T3, S]
                },
                {
                    [],
                    [(RecD < T1, T2, T3, T4 > : TraitB < S > , RecC < T1, T2, T3, T4 > :
                    TraitA < S >),
                    (RecD < T1, T2, T3, T4 > : TraitB < S > , T1 : TraitB < S >),
                    (RecC < T1, T2, T3, T4 > : TraitA < S > ,
                    (T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
                    TraitB < S >), (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Display),
                    (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Default),
                    ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
                    TraitB < S > , T1 : TraitB < S >),
                    ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
                    TraitB < S > ,
                    Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 > : TraitA <
                    S >),
                    ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
                    TraitB < S > , S : Display),
                    ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
                    TraitB < S > , S : Default)], [S, T4, T2, T3, T1]
                }],
                [impl < T1, T2, T3, T4, S > TraitA < S > for RecC < T1, T2, T3, T4 > where
                (T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
                < S > , S : Display + Default,
                {
                    fn get_a(& self) -> String
                    {
                        format!
                        ("RecC: {}",
                        <(T1, Wrapper2<(T2, (T3, (T3, RecD<T1, T2, T3, T4>))), T4>) as
                        TraitB<S>>::get_b(&self.0))
                    }
                }, impl < T1, T2, T3, T4, S > TraitB < S > for RecD < T1, T2, T3, T4 >
                where RecC < T1, T2, T3, T4 > : TraitA < S > , T1 : TraitB < S > ,
                {
                    fn get_b(& self) -> String
                    {
                        if let Some(ref rec_c) = self.0
                        {
                            format!
                            ("RecD {}", <RecC<T1, T2, T3, T4> as TraitA<S>>::get_a(rec_c))
                        } else { format! ("RecD None") }
                    }
                }]
            }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__next_step"
            && e.input == r#""0.2.0", Traitdef
{
    appending_constraints :
    [T1 : TraitB < S > ,
    Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 > : TraitA < S >
    , S : Display + Default]
},
[(T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB < S
>], { :: coinduction }, [TraitB, TraitA], [RecD, RecC],
[{
    [],
    [(RecC < T1, T2, T3, T4 > : TraitA < S > ,
    (T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
    < S >), (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Display),
    (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Default)],
    [T1, T2, S, T3, T4]
},
{
    [],
    [(RecD < T1, T2, T3, T4 > : TraitB < S > , RecC < T1, T2, T3, T4 > :
    TraitA < S >),
    (RecD < T1, T2, T3, T4 > : TraitB < S > , T1 : TraitB < S >),
    (RecC < T1, T2, T3, T4 > : TraitA < S > ,
    (T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
    < S >), (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Display),
    (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Default)],
    [T1, T2, T3, S, T4]
}],
[impl < T1, T2, T3, T4, S > TraitA < S > for RecC < T1, T2, T3, T4 > where
(T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB < S
> , S : Display + Default,
{
    fn get_a(& self) -> String
    {
        format!
        ("RecC: {}",
        <(T1, Wrapper2<(T2, (T3, (T3, RecD<T1, T2, T3, T4>))), T4>) as
        TraitB<S>>::get_b(&self.0))
    }
}, impl < T1, T2, T3, T4, S > TraitB < S > for RecD < T1, T2, T3, T4 > where
RecC < T1, T2, T3, T4 > : TraitA < S > , T1 : TraitB < S > ,
{
    fn get_b(& self) -> String
    {
        if let Some(ref rec_c) = self.0
        {
            format!
            ("RecD {}", <RecC<T1, T2, T3, T4> as TraitA<S>>::get_a(rec_c))
        } else { format! ("RecD None") }
    }
}]"#
            && e.to == r#"TraitA!
{
    "0.2.0", None,
    [Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 > : TraitA < S
    >], { :: coinduction }, [TraitB, TraitA], [RecC, RecD],
    [{
        [],
        [(RecC < T1, T2, T3, T4 > : TraitA < S > ,
        (T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
        TraitB < S >), (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Display),
        (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Default),
        ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
        TraitB < S > , T1 : TraitB < S >),
        ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
        TraitB < S > ,
        Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 > : TraitA <
        S >),
        ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
        TraitB < S > , S : Display),
        ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
        TraitB < S > , S : Default)], [T1, T2, T4, T3, S]
    },
    {
        [],
        [(RecD < T1, T2, T3, T4 > : TraitB < S > , RecC < T1, T2, T3, T4 > :
        TraitA < S >),
        (RecD < T1, T2, T3, T4 > : TraitB < S > , T1 : TraitB < S >),
        (RecC < T1, T2, T3, T4 > : TraitA < S > ,
        (T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
        TraitB < S >), (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Display),
        (RecC < T1, T2, T3, T4 > : TraitA < S > , S : Default),
        ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
        TraitB < S > , T1 : TraitB < S >),
        ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
        TraitB < S > ,
        Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 > : TraitA <
        S >),
        ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
        TraitB < S > , S : Display),
        ((T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) :
        TraitB < S > , S : Default)], [S, T4, T2, T3, T1]
    }],
    [impl < T1, T2, T3, T4, S > TraitA < S > for RecC < T1, T2, T3, T4 > where
    (T1, Wrapper2 < (T2, (T3, (T3, RecD < T1, T2, T3, T4 >))), T4 >) : TraitB
    < S > , S : Display + Default,
    {
        fn get_a(& self) -> String
        {
            format!
            ("RecC: {}",
            <(T1, Wrapper2<(T2, (T3, (T3, RecD<T1, T2, T3, T4>))), T4>) as
            TraitB<S>>::get_b(&self.0))
        }
    }, impl < T1, T2, T3, T4, S > TraitB < S > for RecD < T1, T2, T3, T4 >
    where RecC < T1, T2, T3, T4 > : TraitA < S > , T1 : TraitB < S > ,
    {
        fn get_b(& self) -> String
        {
            if let Some(ref rec_c) = self.0
            {
                format!
                ("RecD {}", <RecC<T1, T2, T3, T4> as TraitA<S>>::get_a(rec_c))
            } else { format! ("RecD None") }
        }
    }]
}"#
    }));
}

#[test]
fn external_crate_coinduction_test_complex_coinduction() {
    let expansions = run_trace_for_repo("coinduction", Some("complex_coinduction"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "traitdef"
            && e.input == r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#
            && e.to == r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_3044150873991545574
{
    ("0.2.0", None, [[$T:ty; $N:expr] :$ ($wt : tt) *],
    { $ ($coinduction : tt) + }, $ ($t : tt) *) =>
    {
        $ ($coinduction) +:: __next_step!
        {
            "0.2.0", Traitdef
            {
                appending_constraints :
                [$T : :: core :: clone :: Clone + :: core :: default ::
                Default]
            }, [[$T; $N] :$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
        }
    };
    ("0.2.0", None, [[$T:ty] :$ ($wt : tt) *], { $ ($coinduction : tt) + }, $
    ($t : tt) *) =>
    {
        $ ($coinduction) +:: __next_step!
        {
            "0.2.0", Traitdef
            {
                appending_constraints :
                [$T : :: core :: cmp :: PartialEq + :: core :: clone :: Clone]
            }, [[$T] :$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
        }
    };
    ("0.2.0", None, [($T:ty, $U:ty) :$ ($wt : tt) *],
    { $ ($coinduction : tt) + }, $ ($t : tt) *) =>
    {
        $ ($coinduction) +:: __next_step!
        {
            "0.2.0", Traitdef
            {
                appending_constraints :
                [$T : :: core :: clone :: Clone + :: core :: marker :: Send,
                $U : :: core :: marker :: Sync + :: core :: default ::
                Default]
            }, [($T, $U) :$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
        }
    };
    ("0.2.0", None, [($T:ty, $U:ty, $V:ty) :$ ($wt : tt) *],
    { $ ($coinduction : tt) + }, $ ($t : tt) *) =>
    {
        $ ($coinduction) +:: __next_step!
        {
            "0.2.0", Traitdef
            {
                appending_constraints :
                [$T : :: core :: clone :: Clone + :: core :: marker :: Send,
                $U : :: core :: marker :: Sync, $V : :: core :: default ::
                Default + :: core :: marker :: Send]
            }, [($T, $U, $V) :$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
        }
    };
    ("0.2.0", None,
    [:: $seg0 : ident $ (:: $segs : ident) * $ (<$ ($arg : ty),*$ (,) ?>) ? :$
    ($wt : tt) *], { $ ($coinduction : tt) + }, $ ($t : tt) *) =>
    {
        :: $seg0 $ (:: $segs) * !
        {
            "0.2.0", None,
            [$ty0 : :: $seg0 $ (:: $segs) * $ (<$ ($arg),*>) ? :$ ($wt) *],
            { $ ($coinduction) + }, $ ($t) *
        }
    };
    ("0.2.0", None,
    [$seg0 : ident $ (:: $segs : ident) * $ (<$ ($arg : ty),*$ (,) ?>) ? :$
    ($wt : tt) *], { $ ($coinduction : tt) + }, $ ($t : tt) *) =>
    {
        $seg0 $ (:: $segs) *!
        {
            "0.2.0", None,
            [$seg0 $ (:: $segs) * $ (<$ ($arg),*>) ? :$ ($wt) *],
            { $ ($coinduction) + }, $ ($t) *
        }
    };
} #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub use
__LocalTrait_temporal_3044150873991545574 as LocalTrait;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "typedef"
            && e.input == r#"pub mod generic_types
{
    use super::*; use std::fmt::Debug; use std::hash::Hash; pub struct
    Container<T, U> { pub first: T, pub second: U, } pub struct Wrapper<T>
    where T: Clone + Debug, { pub value: T, pub count: usize, } pub struct
    MultiGeneric<T, U, V> where T: Clone, U: Send + Sync, V: Debug + Hash,
    { pub primary: T, pub secondary: U, pub metadata: V, } pub struct
    ConstrainedStruct<T> where T: Iterator + Clone, { pub iterator: T, }
    impl<T, U> TestTrait for Container<T, U> where T: Clone +
    ::std::fmt::Debug + Send, U: ::std::fmt::Debug + Default + Sync,
    { fn test_method(&self) -> String { format!("{:?}", self.first) } }
    impl<T> TestTrait for Wrapper<T> where T: Clone + Debug + ToString,
    {
        fn test_method(&self) -> String
        { format!("{}: {}", self.value.to_string(), self.count) }
    } impl<T, U> LocalTrait for Container<T, U> where T: Clone + Send + Sync,
    U: ::std::fmt::Debug + Hash,
    { fn local_method(&self) -> usize { let _ = self.first.clone(); 42 } }
    impl<T> LocalTrait for Wrapper<T> where T: Clone + ::std::fmt::Debug +
    Default,
    { fn local_method(&self) -> usize { let _ = T::default(); self.count } }
    impl<T, U, V> CircularTrait for MultiGeneric<T, U, V> where T: Clone +
    ::std::fmt::Debug + Send + 'static, U: Send + Sync + Default, V:
    ::std::fmt::Debug + Hash + Clone,
    {
        fn circular_method(&self) -> Box<dyn CircularTrait>
        {
            Box::new(ConstrainedStruct
            { iterator: std::iter::once(self.primary.clone()), })
        }
    } impl<T> CircularTrait for ConstrainedStruct<T> where T: Iterator + Clone
    + Send, T::Item: ::std::fmt::Debug,
    {
        fn circular_method(&self) -> Box<dyn CircularTrait>
        {
            Box::new(MultiGeneric
            {
                primary: "circular".to_string(), secondary: 42u32, metadata:
                123usize,
            })
        }
    } impl<T, U> ExtendedTrait for Container<T, U> where T: PartialEq + Clone,
    U: Default + Send,
    {
        fn extended_method(&self) -> bool
        { let _default_u = U::default(); true }
    } impl<T, U, V> ExtendedTrait for MultiGeneric<T, U, V> where T: Clone +
    PartialOrd, U: Send + Sync + Clone, V: ::std::fmt::Debug + Hash + Default,
    {
        fn extended_method(&self) -> bool
        { let _ = V::default(); let _ = self.secondary.clone(); true }
    }
}"#
            && e.to == r#"pub mod generic_types
{
    use super :: * ; use std :: fmt :: Debug; use std :: hash :: Hash; pub
    struct Container < T, U > { pub first : T, pub second : U, } pub struct
    Wrapper < T > where T : Clone + Debug,
    { pub value : T, pub count : usize, } pub struct MultiGeneric < T, U, V >
    where T : Clone, U : Send + Sync, V : Debug + Hash,
    { pub primary : T, pub secondary : U, pub metadata : V, } pub struct
    ConstrainedStruct < T > where T : Iterator + Clone, { pub iterator : T, }
    impl < T, U > TestTrait for Container < T, U > where T : Clone + :: std ::
    fmt :: Debug + Send, U : :: std :: fmt :: Debug + Default + Sync,
    { fn test_method(& self) -> String { format! ("{:?}", self.first) } } impl
    < T > TestTrait for Wrapper < T > where T : Clone + Debug + ToString,
    {
        fn test_method(& self) -> String
        { format! ("{}: {}", self.value.to_string(), self.count) }
    } impl < T, U > LocalTrait for Container < T, U > where T : Clone + Send +
    Sync, U : :: std :: fmt :: Debug + Hash,
    { fn local_method(& self) -> usize { let _ = self.first.clone(); 42 } }
    impl < T > LocalTrait for Wrapper < T > where T : Clone + :: std :: fmt ::
    Debug + Default,
    {
        fn local_method(& self) -> usize
        { let _ = T :: default(); self.count }
    } impl < T, U, V > CircularTrait for MultiGeneric < T, U, V > where T :
    Clone + :: std :: fmt :: Debug + Send + 'static, U : Send + Sync +
    Default, V : :: std :: fmt :: Debug + Hash + Clone,
    {
        fn circular_method(& self) -> Box < dyn CircularTrait >
        {
            Box ::
            new(ConstrainedStruct
            { iterator : std :: iter :: once(self.primary.clone()), })
        }
    } impl < T > CircularTrait for ConstrainedStruct < T > where T : Iterator
    + Clone + Send, T :: Item : :: std :: fmt :: Debug,
    {
        fn circular_method(& self) -> Box < dyn CircularTrait >
        {
            Box ::
            new(MultiGeneric
            {
                primary : "circular".to_string(), secondary : 42u32, metadata
                : 123usize,
            })
        }
    } impl < T, U > ExtendedTrait for Container < T, U > where T : PartialEq +
    Clone, U : Default + Send,
    {
        fn extended_method(& self) -> bool
        { let _default_u = U :: default(); true }
    } impl < T, U, V > ExtendedTrait for MultiGeneric < T, U, V > where T :
    Clone + PartialOrd, U : Send + Sync + Clone, V : :: std :: fmt :: Debug +
    Hash + Default,
    {
        fn extended_method(& self) -> bool
        { let _ = V :: default(); let _ = self.secondary.clone(); true }
    }
    #[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
    #[doc(hidden)] #[macro_export] macro_rules!
    __Container_temporal_13753066654786479059
    {
        ("0.2.0", None, [$ ($wt : tt) *], { $ ($coinduction : tt) + }, $
        ($t : tt) *) =>
        {
            $ ($coinduction) +:: __next_step!
            {
                "0.2.0", Typedef
                {
                    predicates :
                    [([__T_7_13753066654786479059, __U_7_13753066654786479059],
                    Container < __T_7_13753066654786479059,
                    __U_7_13753066654786479059 > : TestTrait,
                    [__T_7_13753066654786479059 : Clone,
                    __T_7_13753066654786479059 : :: std :: fmt :: Debug,
                    __T_7_13753066654786479059 : Send,
                    __U_7_13753066654786479059 : :: std :: fmt :: Debug,
                    __U_7_13753066654786479059 : Default,
                    __U_7_13753066654786479059 : Sync]),
                    ([__T_9_13753066654786479059, __U_9_13753066654786479059],
                    Container < __T_9_13753066654786479059,
                    __U_9_13753066654786479059 > : LocalTrait,
                    [__T_9_13753066654786479059 : Clone,
                    __T_9_13753066654786479059 : Send,
                    __T_9_13753066654786479059 : Sync,
                    __U_9_13753066654786479059 : :: std :: fmt :: Debug,
                    __U_9_13753066654786479059 : Hash]),
                    ([__T_13_13753066654786479059, __U_13_13753066654786479059],
                    Container < __T_13_13753066654786479059,
                    __U_13_13753066654786479059 > : ExtendedTrait,
                    [__T_13_13753066654786479059 : PartialEq,
                    __T_13_13753066654786479059 : Clone,
                    __U_13_13753066654786479059 : Default,
                    __U_13_13753066654786479059 : Send])]
                }, [$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
            }
        }
    } #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub
    use __Container_temporal_13753066654786479059 as Container;
    #[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
    #[doc(hidden)] #[macro_export] macro_rules!
    __Wrapper_temporal_13753066654786479059
    {
        ("0.2.0", None, [$ ($wt : tt) *], { $ ($coinduction : tt) + }, $
        ($t : tt) *) =>
        {
            $ ($coinduction) +:: __next_step!
            {
                "0.2.0", Typedef
                {
                    predicates :
                    [([__T_8_13753066654786479059], Wrapper <
                    __T_8_13753066654786479059 > : TestTrait,
                    [__T_8_13753066654786479059 : Clone,
                    __T_8_13753066654786479059 : Debug,
                    __T_8_13753066654786479059 : ToString]),
                    ([__T_10_13753066654786479059], Wrapper <
                    __T_10_13753066654786479059 > : LocalTrait,
                    [__T_10_13753066654786479059 : Clone,
                    __T_10_13753066654786479059 : :: std :: fmt :: Debug,
                    __T_10_13753066654786479059 : Default])]
                }, [$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
            }
        }
    } #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub
    use __Wrapper_temporal_13753066654786479059 as Wrapper;
    #[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
    #[doc(hidden)] #[macro_export] macro_rules!
    __MultiGeneric_temporal_13753066654786479059
    {
        ("0.2.0", None, [$ ($wt : tt) *], { $ ($coinduction : tt) + }, $
        ($t : tt) *) =>
        {
            $ ($coinduction) +:: __next_step!
            {
                "0.2.0", Typedef
                {
                    predicates :
                    [([__T_11_13753066654786479059, __U_11_13753066654786479059,
                    __V_11_13753066654786479059], MultiGeneric <
                    __T_11_13753066654786479059, __U_11_13753066654786479059,
                    __V_11_13753066654786479059 > : CircularTrait,
                    [__T_11_13753066654786479059 : Clone,
                    __T_11_13753066654786479059 : :: std :: fmt :: Debug,
                    __T_11_13753066654786479059 : Send,
                    __U_11_13753066654786479059 : Send,
                    __U_11_13753066654786479059 : Sync,
                    __U_11_13753066654786479059 : Default,
                    __V_11_13753066654786479059 : :: std :: fmt :: Debug,
                    __V_11_13753066654786479059 : Hash,
                    __V_11_13753066654786479059 : Clone]),
                    ([__T_14_13753066654786479059, __U_14_13753066654786479059,
                    __V_14_13753066654786479059], MultiGeneric <
                    __T_14_13753066654786479059, __U_14_13753066654786479059,
                    __V_14_13753066654786479059 > : ExtendedTrait,
                    [__T_14_13753066654786479059 : Clone,
                    __T_14_13753066654786479059 : PartialOrd,
                    __U_14_13753066654786479059 : Send,
                    __U_14_13753066654786479059 : Sync,
                    __U_14_13753066654786479059 : Clone,
                    __V_14_13753066654786479059 : :: std :: fmt :: Debug,
                    __V_14_13753066654786479059 : Hash,
                    __V_14_13753066654786479059 : Default])]
                }, [$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
            }
        }
    } #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub
    use __MultiGeneric_temporal_13753066654786479059 as MultiGeneric;
    #[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
    #[doc(hidden)] #[macro_export] macro_rules!
    __ConstrainedStruct_temporal_13753066654786479059
    {
        ("0.2.0", None, [$ ($wt : tt) *], { $ ($coinduction : tt) + }, $
        ($t : tt) *) =>
        {
            $ ($coinduction) +:: __next_step!
            {
                "0.2.0", Typedef
                {
                    predicates :
                    [([__T_12_13753066654786479059], ConstrainedStruct <
                    __T_12_13753066654786479059 > : CircularTrait,
                    [__T_12_13753066654786479059 : Iterator,
                    __T_12_13753066654786479059 : Clone,
                    __T_12_13753066654786479059 : Send, T :: Item : :: std ::
                    fmt :: Debug])]
                }, [$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
            }
        }
    } #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub
    use __ConstrainedStruct_temporal_13753066654786479059 as
    ConstrainedStruct;
}"#
    }));
}

#[test]
fn external_crate_coinduction_test_min_calculator() {
    let expansions = run_trace_for_repo("coinduction", Some("min_calculator"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "coinduction"
            && e.input == r#"mod calculator
{
    use super::Evaluate; pub struct Expr; pub struct Term; impl Evaluate for
    Expr where Term: Evaluate,
    {
        fn evaluate(&self, input: &[&'static str], index: &mut usize) -> i32
        {
            let left_val = Term.evaluate(input, index); let op =
            input[*index]; *index += 1; let right_val =
            Term.evaluate(input, index); match op
            {
                "+" => left_val + right_val, "-" => left_val - right_val, _ =>
                left_val,
            }
        }
    } impl Evaluate for Term where Expr: Evaluate,
    {
        fn evaluate(&self, input: &[&'static str], index: &mut usize) -> i32
        {
            let token = input[*index]; *index += 1; if token == "("
            { let result = Expr.evaluate(input, index); *index += 1; result }
            else { token.parse::<i32>().unwrap() }
        }
    }
}"#
            && e.to == r#"mod calculator
{
    use super :: Evaluate; pub struct Expr; pub struct Term; impl Evaluate for
    Expr
    {
        fn evaluate(& self, input : & [& 'static str], index : & mut usize) ->
        i32
        {
            let left_val = Term.evaluate(input, index); let op = input
            [* index]; * index += 1; let right_val =
            Term.evaluate(input, index); match op
            {
                "+" => left_val + right_val, "-" => left_val - right_val, _ =>
                left_val,
            }
        }
    } impl Evaluate for Term
    {
        fn evaluate(& self, input : & [& 'static str], index : & mut usize) ->
        i32
        {
            let token = input [* index]; * index += 1; if token == "("
            { let result = Expr.evaluate(input, index); * index += 1; result }
            else { token.parse :: < i32 > ().unwrap() }
        }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "traitdef"
            && e.input == r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#
            && e.to == r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_3044150873991545574
{
    ("0.2.0", None, [[$T:ty; $N:expr] :$ ($wt : tt) *],
    { $ ($coinduction : tt) + }, $ ($t : tt) *) =>
    {
        $ ($coinduction) +:: __next_step!
        {
            "0.2.0", Traitdef
            {
                appending_constraints :
                [$T : :: core :: clone :: Clone + :: core :: default ::
                Default]
            }, [[$T; $N] :$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
        }
    };
    ("0.2.0", None, [[$T:ty] :$ ($wt : tt) *], { $ ($coinduction : tt) + }, $
    ($t : tt) *) =>
    {
        $ ($coinduction) +:: __next_step!
        {
            "0.2.0", Traitdef
            {
                appending_constraints :
                [$T : :: core :: cmp :: PartialEq + :: core :: clone :: Clone]
            }, [[$T] :$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
        }
    };
    ("0.2.0", None, [($T:ty, $U:ty) :$ ($wt : tt) *],
    { $ ($coinduction : tt) + }, $ ($t : tt) *) =>
    {
        $ ($coinduction) +:: __next_step!
        {
            "0.2.0", Traitdef
            {
                appending_constraints :
                [$T : :: core :: clone :: Clone + :: core :: marker :: Send,
                $U : :: core :: marker :: Sync + :: core :: default ::
                Default]
            }, [($T, $U) :$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
        }
    };
    ("0.2.0", None, [($T:ty, $U:ty, $V:ty) :$ ($wt : tt) *],
    { $ ($coinduction : tt) + }, $ ($t : tt) *) =>
    {
        $ ($coinduction) +:: __next_step!
        {
            "0.2.0", Traitdef
            {
                appending_constraints :
                [$T : :: core :: clone :: Clone + :: core :: marker :: Send,
                $U : :: core :: marker :: Sync, $V : :: core :: default ::
                Default + :: core :: marker :: Send]
            }, [($T, $U, $V) :$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
        }
    };
    ("0.2.0", None,
    [:: $seg0 : ident $ (:: $segs : ident) * $ (<$ ($arg : ty),*$ (,) ?>) ? :$
    ($wt : tt) *], { $ ($coinduction : tt) + }, $ ($t : tt) *) =>
    {
        :: $seg0 $ (:: $segs) * !
        {
            "0.2.0", None,
            [$ty0 : :: $seg0 $ (:: $segs) * $ (<$ ($arg),*>) ? :$ ($wt) *],
            { $ ($coinduction) + }, $ ($t) *
        }
    };
    ("0.2.0", None,
    [$seg0 : ident $ (:: $segs : ident) * $ (<$ ($arg : ty),*$ (,) ?>) ? :$
    ($wt : tt) *], { $ ($coinduction : tt) + }, $ ($t : tt) *) =>
    {
        $seg0 $ (:: $segs) *!
        {
            "0.2.0", None,
            [$seg0 $ (:: $segs) * $ (<$ ($arg),*>) ? :$ ($wt) *],
            { $ ($coinduction) + }, $ ($t) *
        }
    };
} #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub use
__LocalTrait_temporal_3044150873991545574 as LocalTrait;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "typedef"
            && e.input == r#"pub mod generic_types
{
    use super::*; use std::fmt::Debug; use std::hash::Hash; pub struct
    Container<T, U> { pub first: T, pub second: U, } pub struct Wrapper<T>
    where T: Clone + Debug, { pub value: T, pub count: usize, } pub struct
    MultiGeneric<T, U, V> where T: Clone, U: Send + Sync, V: Debug + Hash,
    { pub primary: T, pub secondary: U, pub metadata: V, } pub struct
    ConstrainedStruct<T> where T: Iterator + Clone, { pub iterator: T, }
    impl<T, U> TestTrait for Container<T, U> where T: Clone +
    ::std::fmt::Debug + Send, U: ::std::fmt::Debug + Default + Sync,
    { fn test_method(&self) -> String { format!("{:?}", self.first) } }
    impl<T> TestTrait for Wrapper<T> where T: Clone + Debug + ToString,
    {
        fn test_method(&self) -> String
        { format!("{}: {}", self.value.to_string(), self.count) }
    } impl<T, U> LocalTrait for Container<T, U> where T: Clone + Send + Sync,
    U: ::std::fmt::Debug + Hash,
    { fn local_method(&self) -> usize { let _ = self.first.clone(); 42 } }
    impl<T> LocalTrait for Wrapper<T> where T: Clone + ::std::fmt::Debug +
    Default,
    { fn local_method(&self) -> usize { let _ = T::default(); self.count } }
    impl<T, U, V> CircularTrait for MultiGeneric<T, U, V> where T: Clone +
    ::std::fmt::Debug + Send + 'static, U: Send + Sync + Default, V:
    ::std::fmt::Debug + Hash + Clone,
    {
        fn circular_method(&self) -> Box<dyn CircularTrait>
        {
            Box::new(ConstrainedStruct
            { iterator: std::iter::once(self.primary.clone()), })
        }
    } impl<T> CircularTrait for ConstrainedStruct<T> where T: Iterator + Clone
    + Send, T::Item: ::std::fmt::Debug,
    {
        fn circular_method(&self) -> Box<dyn CircularTrait>
        {
            Box::new(MultiGeneric
            {
                primary: "circular".to_string(), secondary: 42u32, metadata:
                123usize,
            })
        }
    } impl<T, U> ExtendedTrait for Container<T, U> where T: PartialEq + Clone,
    U: Default + Send,
    {
        fn extended_method(&self) -> bool
        { let _default_u = U::default(); true }
    } impl<T, U, V> ExtendedTrait for MultiGeneric<T, U, V> where T: Clone +
    PartialOrd, U: Send + Sync + Clone, V: ::std::fmt::Debug + Hash + Default,
    {
        fn extended_method(&self) -> bool
        { let _ = V::default(); let _ = self.secondary.clone(); true }
    }
}"#
            && e.to == r#"pub mod generic_types
{
    use super :: * ; use std :: fmt :: Debug; use std :: hash :: Hash; pub
    struct Container < T, U > { pub first : T, pub second : U, } pub struct
    Wrapper < T > where T : Clone + Debug,
    { pub value : T, pub count : usize, } pub struct MultiGeneric < T, U, V >
    where T : Clone, U : Send + Sync, V : Debug + Hash,
    { pub primary : T, pub secondary : U, pub metadata : V, } pub struct
    ConstrainedStruct < T > where T : Iterator + Clone, { pub iterator : T, }
    impl < T, U > TestTrait for Container < T, U > where T : Clone + :: std ::
    fmt :: Debug + Send, U : :: std :: fmt :: Debug + Default + Sync,
    { fn test_method(& self) -> String { format! ("{:?}", self.first) } } impl
    < T > TestTrait for Wrapper < T > where T : Clone + Debug + ToString,
    {
        fn test_method(& self) -> String
        { format! ("{}: {}", self.value.to_string(), self.count) }
    } impl < T, U > LocalTrait for Container < T, U > where T : Clone + Send +
    Sync, U : :: std :: fmt :: Debug + Hash,
    { fn local_method(& self) -> usize { let _ = self.first.clone(); 42 } }
    impl < T > LocalTrait for Wrapper < T > where T : Clone + :: std :: fmt ::
    Debug + Default,
    {
        fn local_method(& self) -> usize
        { let _ = T :: default(); self.count }
    } impl < T, U, V > CircularTrait for MultiGeneric < T, U, V > where T :
    Clone + :: std :: fmt :: Debug + Send + 'static, U : Send + Sync +
    Default, V : :: std :: fmt :: Debug + Hash + Clone,
    {
        fn circular_method(& self) -> Box < dyn CircularTrait >
        {
            Box ::
            new(ConstrainedStruct
            { iterator : std :: iter :: once(self.primary.clone()), })
        }
    } impl < T > CircularTrait for ConstrainedStruct < T > where T : Iterator
    + Clone + Send, T :: Item : :: std :: fmt :: Debug,
    {
        fn circular_method(& self) -> Box < dyn CircularTrait >
        {
            Box ::
            new(MultiGeneric
            {
                primary : "circular".to_string(), secondary : 42u32, metadata
                : 123usize,
            })
        }
    } impl < T, U > ExtendedTrait for Container < T, U > where T : PartialEq +
    Clone, U : Default + Send,
    {
        fn extended_method(& self) -> bool
        { let _default_u = U :: default(); true }
    } impl < T, U, V > ExtendedTrait for MultiGeneric < T, U, V > where T :
    Clone + PartialOrd, U : Send + Sync + Clone, V : :: std :: fmt :: Debug +
    Hash + Default,
    {
        fn extended_method(& self) -> bool
        { let _ = V :: default(); let _ = self.secondary.clone(); true }
    }
    #[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
    #[doc(hidden)] #[macro_export] macro_rules!
    __Container_temporal_13753066654786479059
    {
        ("0.2.0", None, [$ ($wt : tt) *], { $ ($coinduction : tt) + }, $
        ($t : tt) *) =>
        {
            $ ($coinduction) +:: __next_step!
            {
                "0.2.0", Typedef
                {
                    predicates :
                    [([__T_7_13753066654786479059, __U_7_13753066654786479059],
                    Container < __T_7_13753066654786479059,
                    __U_7_13753066654786479059 > : TestTrait,
                    [__T_7_13753066654786479059 : Clone,
                    __T_7_13753066654786479059 : :: std :: fmt :: Debug,
                    __T_7_13753066654786479059 : Send,
                    __U_7_13753066654786479059 : :: std :: fmt :: Debug,
                    __U_7_13753066654786479059 : Default,
                    __U_7_13753066654786479059 : Sync]),
                    ([__T_9_13753066654786479059, __U_9_13753066654786479059],
                    Container < __T_9_13753066654786479059,
                    __U_9_13753066654786479059 > : LocalTrait,
                    [__T_9_13753066654786479059 : Clone,
                    __T_9_13753066654786479059 : Send,
                    __T_9_13753066654786479059 : Sync,
                    __U_9_13753066654786479059 : :: std :: fmt :: Debug,
                    __U_9_13753066654786479059 : Hash]),
                    ([__T_13_13753066654786479059, __U_13_13753066654786479059],
                    Container < __T_13_13753066654786479059,
                    __U_13_13753066654786479059 > : ExtendedTrait,
                    [__T_13_13753066654786479059 : PartialEq,
                    __T_13_13753066654786479059 : Clone,
                    __U_13_13753066654786479059 : Default,
                    __U_13_13753066654786479059 : Send])]
                }, [$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
            }
        }
    } #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub
    use __Container_temporal_13753066654786479059 as Container;
    #[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
    #[doc(hidden)] #[macro_export] macro_rules!
    __Wrapper_temporal_13753066654786479059
    {
        ("0.2.0", None, [$ ($wt : tt) *], { $ ($coinduction : tt) + }, $
        ($t : tt) *) =>
        {
            $ ($coinduction) +:: __next_step!
            {
                "0.2.0", Typedef
                {
                    predicates :
                    [([__T_8_13753066654786479059], Wrapper <
                    __T_8_13753066654786479059 > : TestTrait,
                    [__T_8_13753066654786479059 : Clone,
                    __T_8_13753066654786479059 : Debug,
                    __T_8_13753066654786479059 : ToString]),
                    ([__T_10_13753066654786479059], Wrapper <
                    __T_10_13753066654786479059 > : LocalTrait,
                    [__T_10_13753066654786479059 : Clone,
                    __T_10_13753066654786479059 : :: std :: fmt :: Debug,
                    __T_10_13753066654786479059 : Default])]
                }, [$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
            }
        }
    } #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub
    use __Wrapper_temporal_13753066654786479059 as Wrapper;
    #[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
    #[doc(hidden)] #[macro_export] macro_rules!
    __MultiGeneric_temporal_13753066654786479059
    {
        ("0.2.0", None, [$ ($wt : tt) *], { $ ($coinduction : tt) + }, $
        ($t : tt) *) =>
        {
            $ ($coinduction) +:: __next_step!
            {
                "0.2.0", Typedef
                {
                    predicates :
                    [([__T_11_13753066654786479059, __U_11_13753066654786479059,
                    __V_11_13753066654786479059], MultiGeneric <
                    __T_11_13753066654786479059, __U_11_13753066654786479059,
                    __V_11_13753066654786479059 > : CircularTrait,
                    [__T_11_13753066654786479059 : Clone,
                    __T_11_13753066654786479059 : :: std :: fmt :: Debug,
                    __T_11_13753066654786479059 : Send,
                    __U_11_13753066654786479059 : Send,
                    __U_11_13753066654786479059 : Sync,
                    __U_11_13753066654786479059 : Default,
                    __V_11_13753066654786479059 : :: std :: fmt :: Debug,
                    __V_11_13753066654786479059 : Hash,
                    __V_11_13753066654786479059 : Clone]),
                    ([__T_14_13753066654786479059, __U_14_13753066654786479059,
                    __V_14_13753066654786479059], MultiGeneric <
                    __T_14_13753066654786479059, __U_14_13753066654786479059,
                    __V_14_13753066654786479059 > : ExtendedTrait,
                    [__T_14_13753066654786479059 : Clone,
                    __T_14_13753066654786479059 : PartialOrd,
                    __U_14_13753066654786479059 : Send,
                    __U_14_13753066654786479059 : Sync,
                    __U_14_13753066654786479059 : Clone,
                    __V_14_13753066654786479059 : :: std :: fmt :: Debug,
                    __V_14_13753066654786479059 : Hash,
                    __V_14_13753066654786479059 : Default])]
                }, [$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
            }
        }
    } #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub
    use __MultiGeneric_temporal_13753066654786479059 as MultiGeneric;
    #[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
    #[doc(hidden)] #[macro_export] macro_rules!
    __ConstrainedStruct_temporal_13753066654786479059
    {
        ("0.2.0", None, [$ ($wt : tt) *], { $ ($coinduction : tt) + }, $
        ($t : tt) *) =>
        {
            $ ($coinduction) +:: __next_step!
            {
                "0.2.0", Typedef
                {
                    predicates :
                    [([__T_12_13753066654786479059], ConstrainedStruct <
                    __T_12_13753066654786479059 > : CircularTrait,
                    [__T_12_13753066654786479059 : Iterator,
                    __T_12_13753066654786479059 : Clone,
                    __T_12_13753066654786479059 : Send, T :: Item : :: std ::
                    fmt :: Debug])]
                }, [$ ($wt) *], { $ ($coinduction) + }, $ ($t) *
            }
        }
    } #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub
    use __ConstrainedStruct_temporal_13753066654786479059 as
    ConstrainedStruct;
}"#
    }));
}

#[test]
fn external_crate_decycle_show_expansion() {
    let expansions = run_trace_for_repo("decycle", None);
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "name"
            && e.input == r#""get_cell""#
            && e.to == r#"name(& format! ("get_cell"))"#
    }));
}

#[test]
fn external_crate_decycle_test_advanced_cycles() {
    let expansions = run_trace_for_repo("decycle", Some("advanced_cycles"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "decycle"
            && e.input == r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#
            && e.to == r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_9874485626140785372
{
    ("decycle" "0.3.0" [$_ : path, $wl1 : path $ (,$wl : path) * $ (,) ?]
    { $ ($trait_defs : tt) * } $ ($t : tt) *) =>
    {
        $wl1!
        {
            "decycle" "0.3.0" [$wl1 $ (,$wl) *]
            {
                pub trait LocalTrait { fn local_method(& self) -> usize; }, $
                ($trait_defs) *
            } $ ($t) *
        }
    };
} #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub use
__LocalTrait_temporal_9874485626140785372 as LocalTrait;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "name"
            && e.input == r#""get_cell""#
            && e.to == r#"name(& format! ("get_cell"))"#
    }));
}

#[test]
fn external_crate_decycle_test_bug2() {
    let expansions = run_trace_for_repo("decycle", Some("bug2"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "decycle"
            && e.input == r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#
            && e.to == r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_9874485626140785372
{
    ("decycle" "0.3.0" [$_ : path, $wl1 : path $ (,$wl : path) * $ (,) ?]
    { $ ($trait_defs : tt) * } $ ($t : tt) *) =>
    {
        $wl1!
        {
            "decycle" "0.3.0" [$wl1 $ (,$wl) *]
            {
                pub trait LocalTrait { fn local_method(& self) -> usize; }, $
                ($trait_defs) *
            } $ ($t) *
        }
    };
} #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub use
__LocalTrait_temporal_9874485626140785372 as LocalTrait;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__MyTrait_temporal_9874485626140785372"
            && e.input == r#""decycle" "0.3.0" [MyTrait, :: decycle :: __finalize] {}
           {
               impl < 'a, 'b, const N : usize, T > MyTrait < 'a > for MyStruct < 'a, 'b,
               N, T >
               {
                   type MyTrait = T; type T = T; fn f < 'c > (& 'a self, i : & 'c [u8])
                   -> usize { 0 }
               }
           } 10usize true"#
            && e.to == r#":: decycle :: __finalize !
           {
               "decycle" "0.3.0" [:: decycle :: __finalize]
               {
                   #[allow(dead_code)] pub trait MyTrait < 'a9874485626140785372 >
                   {
                       type MyTrait; type T; fn f < 'b >
                       (& 'a9874485626140785372 self, _ : & 'b [u8]) -> usize { 0 }
                   },
               }
               {
                   impl < 'a, 'b, const N : usize, T > MyTrait < 'a > for MyStruct < 'a,
                   'b, N, T >
                   {
                       type MyTrait = T; type T = T; fn f < 'c >
                       (& 'a self, i : & 'c [u8]) -> usize { 0 }
                   }
               } 10usize true
           }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__finalize"
            && e.input == r#""decycle" "0.3.0" [:: decycle :: __finalize]
{
    #[allow(dead_code)] pub trait MyTrait < 'a9874485626140785372 >
    {
        type MyTrait; type T; fn f < 'b >
        (& 'a9874485626140785372 self, _ : & 'b [u8]) -> usize { 0 }
    },
}
{
    impl < 'a, 'b, const N : usize, T > MyTrait < 'a > for MyStruct < 'a, 'b,
    N, T >
    {
        type MyTrait = T; type T = T; fn f < 'c > (& 'a self, i : & 'c [u8])
        -> usize { 0 }
    }
} 10usize true"#
            && e.to == r#"#[doc(hidden)] mod shadowing_module9874485626140785372
{
    trait GetVTableKey9874485626140785372
    {
        extern "C" fn get_vtable_key9874485626140785372(& self) {} fn
        get_cell9874485626140785372(id : :: core :: primitive :: usize) -> &
        'static :: std :: sync :: OnceLock <:: core :: primitive :: usize >
        {
            use :: std :: sync :: { Mutex, OnceLock }; use :: std ::
            collections :: HashMap; use :: std :: primitive ::*; static
            VTABLE_MAP9874485626140785372 : OnceLock < Mutex < HashMap <
            (usize, usize), OnceLock < usize >>>> = OnceLock :: new(); let map
            =
            VTABLE_MAP9874485626140785372.get_or_init(|| Mutex ::
            new(HashMap :: new())); let mut map = map.lock().unwrap(); let r =
            map.entry((Self ::get_vtable_key9874485626140785372 as usize,
            id)).or_insert(OnceLock :: new()); unsafe
            { :: core :: mem :: transmute(r) }
        }
    } impl < T : ?:: core :: marker :: Sized > GetVTableKey9874485626140785372
    for T {} pub mod ranked_traits9874485626140785372
    {
        #[allow(unused)] use super :: super ::*; #[allow(unused)] use super
        ::GetVTableKey9874485626140785372; #[allow(unused)] #[doc(hidden)] pub
        trait MyTraitRanked9874485626140785372 < 'a9874485626140785372,
        Rank9874485626140785372 >
        {
            type MyTrait; type T; fn f < 'b >
            (& 'a9874485626140785372 self, _ : & 'b [u8]) -> usize;
        } #[allow(unused_variables)] impl < 'a, 'b, const N : usize, T >
        MyTraitRanked9874485626140785372 < 'a, () > for MyStruct < 'a, 'b, N,
        T >
        {
            type MyTrait = T; type T = T; fn f < 'c >
            (& 'a self, i : & 'c [u8]) -> usize
            {
                #[allow(unused_unsafe)] unsafe
                {
                    :: core :: mem :: transmute ::< _, fn(& 'a Self, & 'c [u8])
                    -> usize >
                    (Self ::get_cell9874485626140785372(2usize).get().unwrap())
                    (self, i)
                }
            }
        }
    } #[allow(unused)] use super ::*; #[allow(non_camel_case_types)] trait
    MyTrait {} #[allow(unused)] use ranked_traits9874485626140785372 ::*;
    #[allow(unused)] use super :: super ::*;
    #[allow(unused_variables, unused_unsafe)] impl < 'a, 'b, const N : usize,
    T, Rank9874485626140785372 > ranked_traits9874485626140785372 ::
    MyTraitRanked9874485626140785372 < 'a, (Rank9874485626140785372,) > for
    MyStruct < 'a, 'b, N, T > where Self : ranked_traits9874485626140785372 ::
    MyTraitRanked9874485626140785372 < 'a, Rank9874485626140785372 >
    {
        type MyTrait = T; type T = T; fn f < 'c > (& 'a self, i : & 'c [u8])
        -> usize
        {
            let _ = Self ::
            get_cell9874485626140785372(2usize).set(< Self as
            ranked_traits9874485626140785372 ::
            MyTraitRanked9874485626140785372 < 'a, Rank9874485626140785372 > >
            :: f as _); { 0 }
        }
    }
} impl < 'a, 'b, const N : usize, T > MyTrait < 'a > for MyStruct < 'a, 'b, N,
T > where Self : shadowing_module9874485626140785372 ::
ranked_traits9874485626140785372 :: MyTraitRanked9874485626140785372 < 'a,
(((((((((((),),),),),),),),),),) >
{
    type MyTrait = < Self as shadowing_module9874485626140785372
    ::ranked_traits9874485626140785372 ::MyTraitRanked9874485626140785372 <
    'a, (((((((((((),),),),),),),),),),) > > ::MyTrait; type T = < Self as
    shadowing_module9874485626140785372 ::ranked_traits9874485626140785372
    ::MyTraitRanked9874485626140785372 < 'a, (((((((((((),),),),),),),),),),)
    > > ::T; fn f < 'c > (& 'a self, i : & 'c [u8]) -> usize
    {
        < Self as shadowing_module9874485626140785372
        ::ranked_traits9874485626140785372 ::MyTraitRanked9874485626140785372
        < 'a, (((((((((((),),),),),),),),),),) > > ::f(self, i)
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "name"
            && e.input == r#""get_cell""#
            && e.to == r#"name(& format! ("get_cell"))"#
    }));
}

#[test]
fn external_crate_decycle_test_bug3() {
    let expansions = run_trace_for_repo("decycle", Some("bug3"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "decycle"
            && e.input == r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#
            && e.to == r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_9874485626140785372
{
    ("decycle" "0.3.0" [$_ : path, $wl1 : path $ (,$wl : path) * $ (,) ?]
    { $ ($trait_defs : tt) * } $ ($t : tt) *) =>
    {
        $wl1!
        {
            "decycle" "0.3.0" [$wl1 $ (,$wl) *]
            {
                pub trait LocalTrait { fn local_method(& self) -> usize; }, $
                ($trait_defs) *
            } $ ($t) *
        }
    };
} #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub use
__LocalTrait_temporal_9874485626140785372 as LocalTrait;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__Unparse_temporal_9874485626140785372"
            && e.input == r#""decycle" "0.3.0" [Unparse, :: decycle :: __finalize] {}
          {
              impl Unparse for S
              {
                  fn unparse(& self, i : usize)
                  { if i == 0 { return; } < _ as Unparse > :: unparse(self, i - 1); }
              }
          } 10usize true"#
            && e.to == r#":: decycle :: __finalize !
          {
              "decycle" "0.3.0" [:: decycle :: __finalize]
              { #[allow(unused)] trait Unparse { fn unparse(& self, _ : usize); }, }
              {
                  impl Unparse for S
                  {
                      fn unparse(& self, i : usize)
                      {
                          if i == 0 { return; } < _ as Unparse > ::
                          unparse(self, i - 1);
                      }
                  }
              } 10usize true
          }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__finalize"
            && e.input == r#""decycle" "0.3.0" [:: decycle :: __finalize]
{ #[allow(unused)] trait Unparse { fn unparse(& self, _ : usize); }, }
{
    impl Unparse for S
    {
        fn unparse(& self, i : usize)
        { if i == 0 { return; } < _ as Unparse > :: unparse(self, i - 1); }
    }
} 10usize true"#
            && e.to == r#"#[doc(hidden)] mod shadowing_module9874485626140785372
{
    trait GetVTableKey9874485626140785372
    {
        extern "C" fn get_vtable_key9874485626140785372(& self) {} fn
        get_cell9874485626140785372(id : :: core :: primitive :: usize) -> &
        'static :: std :: sync :: OnceLock <:: core :: primitive :: usize >
        {
            use :: std :: sync :: { Mutex, OnceLock }; use :: std ::
            collections :: HashMap; use :: std :: primitive ::*; static
            VTABLE_MAP9874485626140785372 : OnceLock < Mutex < HashMap <
            (usize, usize), OnceLock < usize >>>> = OnceLock :: new(); let map
            =
            VTABLE_MAP9874485626140785372.get_or_init(|| Mutex ::
            new(HashMap :: new())); let mut map = map.lock().unwrap(); let r =
            map.entry((Self ::get_vtable_key9874485626140785372 as usize,
            id)).or_insert(OnceLock :: new()); unsafe
            { :: core :: mem :: transmute(r) }
        }
    } impl < T : ?:: core :: marker :: Sized > GetVTableKey9874485626140785372
    for T {} pub mod ranked_traits9874485626140785372
    {
        #[allow(unused)] use super :: super ::*; #[allow(unused)] use super
        ::GetVTableKey9874485626140785372; #[allow(unused)] #[doc(hidden)] pub
        trait UnparseRanked9874485626140785372 < Rank9874485626140785372 >
        { fn unparse(& self, _ : usize); } #[allow(unused_variables)] impl
        UnparseRanked9874485626140785372 < () > for S
        {
            fn unparse(& self, i : usize)
            {
                #[allow(unused_unsafe)] unsafe
                {
                    :: core :: mem :: transmute ::< _, fn(& Self, usize) >
                    (Self ::get_cell9874485626140785372(0usize).get().unwrap())
                    (self, i)
                }
            }
        }
    } #[allow(unused)] use super ::*; #[allow(non_camel_case_types)] trait
    Unparse {} #[allow(unused)] use ranked_traits9874485626140785372 ::*;
    #[allow(unused)] use super :: super ::*;
    #[allow(unused_variables, unused_unsafe)] impl < Rank9874485626140785372 >
    ranked_traits9874485626140785372 :: UnparseRanked9874485626140785372 <
    (Rank9874485626140785372,) > for S where Self :
    ranked_traits9874485626140785372 :: UnparseRanked9874485626140785372 <
    Rank9874485626140785372 >
    {
        fn unparse(& self, i : usize)
        {
            let _ = Self ::
            get_cell9874485626140785372(0usize).set(< Self as
            ranked_traits9874485626140785372 ::
            UnparseRanked9874485626140785372 < Rank9874485626140785372 > > ::
            unparse as _);
            {
                if i == 0 { return; } < _ as ranked_traits9874485626140785372
                :: UnparseRanked9874485626140785372 < Rank9874485626140785372
                > > :: unparse(self, i - 1);
            }
        }
    }
} impl Unparse for S where Self : shadowing_module9874485626140785372 ::
ranked_traits9874485626140785372 :: UnparseRanked9874485626140785372 <
(((((((((((),),),),),),),),),),) >
{
    fn unparse(& self, i : usize)
    {
        < Self as shadowing_module9874485626140785372
        ::ranked_traits9874485626140785372 ::UnparseRanked9874485626140785372
        < (((((((((((),),),),),),),),),),) > > ::unparse(self, i)
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "name"
            && e.input == r#""get_cell""#
            && e.to == r#"name(& format! ("get_cell"))"#
    }));
}

#[test]
fn external_crate_decycle_test_bug4() {
    let expansions = run_trace_for_repo("decycle", Some("bug4"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "decycle"
            && e.input == r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#
            && e.to == r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_9874485626140785372
{
    ("decycle" "0.3.0" [$_ : path, $wl1 : path $ (,$wl : path) * $ (,) ?]
    { $ ($trait_defs : tt) * } $ ($t : tt) *) =>
    {
        $wl1!
        {
            "decycle" "0.3.0" [$wl1 $ (,$wl) *]
            {
                pub trait LocalTrait { fn local_method(& self) -> usize; }, $
                ($trait_defs) *
            } $ ($t) *
        }
    };
} #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub use
__LocalTrait_temporal_9874485626140785372 as LocalTrait;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__Unparse_temporal_9874485626140785372"
            && e.input == r#""decycle" "0.3.0" [Unparse, :: decycle :: __finalize] {}
           {
               impl < __A > Unparse < __A > for ItemMod
               {
                   fn unparse < B : crate :: TraitA < __A, S = B > > (_ : & mut B) {} fn
                   f(_sink : impl TraitA < __A, S = __A >) {}
               }
           } 10usize true"#
            && e.to == r#":: decycle :: __finalize !
           {
               "decycle" "0.3.0" [:: decycle :: __finalize]
               {
                   pub trait Unparse < A9874485626140785372 >
                   {
                       fn unparse < S : crate :: TraitA < A9874485626140785372, S = S > >
                       (sink : & mut S); fn
                       f(sink : impl crate :: TraitA < A9874485626140785372, S =
                       A9874485626140785372 >);
                   },
               }
               {
                   impl < __A > Unparse < __A > for ItemMod
                   {
                       fn unparse < B : crate :: TraitA < __A, S = B > > (_ : & mut B) {}
                       fn f(_sink : impl TraitA < __A, S = __A >) {}
                   }
               } 10usize true
           }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__finalize"
            && e.input == r#""decycle" "0.3.0" [:: decycle :: __finalize]
{
    pub trait Unparse < A9874485626140785372 >
    {
        fn unparse < S : crate :: TraitA < A9874485626140785372, S = S > >
        (sink : & mut S); fn
        f(sink : impl crate :: TraitA < A9874485626140785372, S =
        A9874485626140785372 >);
    },
}
{
    impl < __A > Unparse < __A > for ItemMod
    {
        fn unparse < B : crate :: TraitA < __A, S = B > > (_ : & mut B) {} fn
        f(_sink : impl TraitA < __A, S = __A >) {}
    }
} 10usize true"#
            && e.to == r#"#[doc(hidden)] mod shadowing_module9874485626140785372
{
    trait GetVTableKey9874485626140785372
    {
        extern "C" fn get_vtable_key9874485626140785372(& self) {} fn
        get_cell9874485626140785372(id : :: core :: primitive :: usize) -> &
        'static :: std :: sync :: OnceLock <:: core :: primitive :: usize >
        {
            use :: std :: sync :: { Mutex, OnceLock }; use :: std ::
            collections :: HashMap; use :: std :: primitive ::*; static
            VTABLE_MAP9874485626140785372 : OnceLock < Mutex < HashMap <
            (usize, usize), OnceLock < usize >>>> = OnceLock :: new(); let map
            =
            VTABLE_MAP9874485626140785372.get_or_init(|| Mutex ::
            new(HashMap :: new())); let mut map = map.lock().unwrap(); let r =
            map.entry((Self ::get_vtable_key9874485626140785372 as usize,
            id)).or_insert(OnceLock :: new()); unsafe
            { :: core :: mem :: transmute(r) }
        }
    } impl < T : ?:: core :: marker :: Sized > GetVTableKey9874485626140785372
    for T {} pub mod ranked_traits9874485626140785372
    {
        #[allow(unused)] use super :: super ::*; #[allow(unused)] use super
        ::GetVTableKey9874485626140785372; #[allow(unused)] #[doc(hidden)] pub
        trait UnparseRanked9874485626140785372 < Rank9874485626140785372,
        A9874485626140785372 >
        {
            fn unparse < S : crate :: TraitA < A9874485626140785372, S = S > >
            (sink : & mut S); fn f < ImplTrait09874485626140785372 : crate ::
            TraitA < A9874485626140785372, S = A9874485626140785372 > >
            (sink : ImplTrait09874485626140785372);
        } #[allow(unused_variables)] impl < __A >
        UnparseRanked9874485626140785372 < (), __A > for ItemMod
        {
            fn unparse < B : crate :: TraitA < __A, S = B > >
            (__arg_0_ : & mut B)
            {
                #[allow(unused_unsafe)] unsafe
                {
                    :: core :: mem :: transmute ::< _, fn(& mut B) >
                    (Self ::get_cell9874485626140785372(0usize).get().unwrap())
                    (__arg_0_)
                }
            } fn f < ImplTrait09874485626140785372 : TraitA < __A, S = __A > >
            (_sink : ImplTrait09874485626140785372)
            {
                #[allow(unused_unsafe)] unsafe
                {
                    :: core :: mem :: transmute ::< _,
                    fn(ImplTrait09874485626140785372) >
                    (Self ::get_cell9874485626140785372(1usize).get().unwrap())
                    (_sink)
                }
            }
        }
    } #[allow(unused)] use super ::*; #[allow(non_camel_case_types)] trait
    Unparse {} #[allow(unused)] use ranked_traits9874485626140785372 ::*;
    #[allow(unused)] use super :: super ::*;
    #[allow(unused_variables, unused_unsafe)] impl < __A,
    Rank9874485626140785372 > ranked_traits9874485626140785372 ::
    UnparseRanked9874485626140785372 < (Rank9874485626140785372,), __A > for
    ItemMod where Self : ranked_traits9874485626140785372 ::
    UnparseRanked9874485626140785372 < Rank9874485626140785372, __A >
    {
        fn unparse < B : crate :: TraitA < __A, S = B > > (_ : & mut B)
        {
            let _ = Self ::
            get_cell9874485626140785372(0usize).set(< Self as
            ranked_traits9874485626140785372 ::
            UnparseRanked9874485626140785372 < Rank9874485626140785372, __A >
            > :: unparse :: < B > as _); {}
        } fn f < ImplTrait09874485626140785372 : TraitA < __A, S = __A > >
        (_sink : ImplTrait09874485626140785372)
        {
            let _ = Self ::
            get_cell9874485626140785372(1usize).set(< Self as
            ranked_traits9874485626140785372 ::
            UnparseRanked9874485626140785372 < Rank9874485626140785372, __A >
            > :: f :: < ImplTrait09874485626140785372 > as _); {}
        }
    }
} impl < __A > Unparse < __A > for ItemMod where Self :
shadowing_module9874485626140785372 :: ranked_traits9874485626140785372 ::
UnparseRanked9874485626140785372 < (((((((((((),),),),),),),),),),), __A >
{
    fn unparse < B : crate :: TraitA < __A, S = B > > (__arg_0_ : & mut B)
    {
        < Self as shadowing_module9874485626140785372
        ::ranked_traits9874485626140785372 ::UnparseRanked9874485626140785372
        < (((((((((((),),),),),),),),),),), __A > > ::unparse(__arg_0_)
    } fn f(_sink : impl TraitA < __A, S = __A >)
    {
        < Self as shadowing_module9874485626140785372
        ::ranked_traits9874485626140785372 ::UnparseRanked9874485626140785372
        < (((((((((((),),),),),),),),),),), __A > > ::f(_sink)
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "name"
            && e.input == r#""get_cell""#
            && e.to == r#"name(& format! ("get_cell"))"#
    }));
}

#[test]
fn external_crate_decycle_test_bug5() {
    let expansions = run_trace_for_repo("decycle", Some("bug5"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "decycle"
            && e.input == r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#
            && e.to == r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_9874485626140785372
{
    ("decycle" "0.3.0" [$_ : path, $wl1 : path $ (,$wl : path) * $ (,) ?]
    { $ ($trait_defs : tt) * } $ ($t : tt) *) =>
    {
        $wl1!
        {
            "decycle" "0.3.0" [$wl1 $ (,$wl) *]
            {
                pub trait LocalTrait { fn local_method(& self) -> usize; }, $
                ($trait_defs) *
            } $ ($t) *
        }
    };
} #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub use
__LocalTrait_temporal_9874485626140785372 as LocalTrait;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__Parse_temporal_9874485626140785372"
            && e.input == r#""decycle" "0.3.0" [Parse, :: decycle :: __finalize] {}
          {
              impl < Item > Parse < Item > for S
              {
                  fn parse < I : :: core :: iter :: Iterator < Item = Item > > (_ : I)
                  { todo! () }
              }
          } 10usize true"#
            && e.to == r#":: decycle :: __finalize !
          {
              "decycle" "0.3.0" [:: decycle :: __finalize]
              {
                  pub trait Parse < Item9874485626140785372 > : :: core :: marker ::
                  Sized
                  {
                      fn parse < I : :: core :: iter :: Iterator < Item =
                      Item9874485626140785372 > > (stream : I);
                  },
              }
              {
                  impl < Item > Parse < Item > for S
                  {
                      fn parse < I : :: core :: iter :: Iterator < Item = Item > >
                      (_ : I) { todo! () }
                  }
              } 10usize true
          }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__finalize"
            && e.input == r#""decycle" "0.3.0" [:: decycle :: __finalize]
{
    pub trait Parse < Item9874485626140785372 > : :: core :: marker :: Sized
    {
        fn parse < I : :: core :: iter :: Iterator < Item =
        Item9874485626140785372 > > (stream : I);
    },
}
{
    impl < Item > Parse < Item > for S
    {
        fn parse < I : :: core :: iter :: Iterator < Item = Item > > (_ : I)
        { todo! () }
    }
} 10usize true"#
            && e.to == r#"#[doc(hidden)] mod shadowing_module9874485626140785372
{
    trait GetVTableKey9874485626140785372
    {
        extern "C" fn get_vtable_key9874485626140785372(& self) {} fn
        get_cell9874485626140785372(id : :: core :: primitive :: usize) -> &
        'static :: std :: sync :: OnceLock <:: core :: primitive :: usize >
        {
            use :: std :: sync :: { Mutex, OnceLock }; use :: std ::
            collections :: HashMap; use :: std :: primitive ::*; static
            VTABLE_MAP9874485626140785372 : OnceLock < Mutex < HashMap <
            (usize, usize), OnceLock < usize >>>> = OnceLock :: new(); let map
            =
            VTABLE_MAP9874485626140785372.get_or_init(|| Mutex ::
            new(HashMap :: new())); let mut map = map.lock().unwrap(); let r =
            map.entry((Self ::get_vtable_key9874485626140785372 as usize,
            id)).or_insert(OnceLock :: new()); unsafe
            { :: core :: mem :: transmute(r) }
        }
    } impl < T : ?:: core :: marker :: Sized > GetVTableKey9874485626140785372
    for T {} pub mod ranked_traits9874485626140785372
    {
        #[allow(unused)] use super :: super ::*; #[allow(unused)] use super
        ::GetVTableKey9874485626140785372; #[allow(unused)] #[doc(hidden)] pub
        trait ParseRanked9874485626140785372 < Rank9874485626140785372,
        Item9874485626140785372 > : :: core :: marker :: Sized
        {
            fn parse < I : :: core :: iter :: Iterator < Item =
            Item9874485626140785372 > > (stream : I);
        } #[allow(unused_variables)] impl < Item >
        ParseRanked9874485626140785372 < (), Item > for S
        {
            fn parse < I : :: core :: iter :: Iterator < Item = Item > >
            (__arg_0_ : I)
            {
                #[allow(unused_unsafe)] unsafe
                {
                    :: core :: mem :: transmute ::< _, fn(I) >
                    (Self ::get_cell9874485626140785372(0usize).get().unwrap())
                    (__arg_0_)
                }
            }
        }
    } #[allow(unused)] use super ::*; #[allow(non_camel_case_types)] trait
    Parse {} #[allow(unused)] use ranked_traits9874485626140785372 ::*;
    #[allow(unused)] use super :: super ::*;
    #[allow(unused_variables, unused_unsafe)] impl < Item,
    Rank9874485626140785372 > ranked_traits9874485626140785372 ::
    ParseRanked9874485626140785372 < (Rank9874485626140785372,), Item > for S
    where Self : ranked_traits9874485626140785372 ::
    ParseRanked9874485626140785372 < Rank9874485626140785372, Item >
    {
        fn parse < I : :: core :: iter :: Iterator < Item = Item > > (_ : I)
        {
            let _ = Self ::
            get_cell9874485626140785372(0usize).set(< Self as
            ranked_traits9874485626140785372 :: ParseRanked9874485626140785372
            < Rank9874485626140785372, Item > > :: parse :: < I > as _);
            { todo! () }
        }
    }
} impl < Item > Parse < Item > for S where Self :
shadowing_module9874485626140785372 :: ranked_traits9874485626140785372 ::
ParseRanked9874485626140785372 < (((((((((((),),),),),),),),),),), Item >
{
    fn parse < I : :: core :: iter :: Iterator < Item = Item > >
    (__arg_0_ : I)
    {
        < Self as shadowing_module9874485626140785372
        ::ranked_traits9874485626140785372 ::ParseRanked9874485626140785372 <
        (((((((((((),),),),),),),),),),), Item > > ::parse(__arg_0_)
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "name"
            && e.input == r#""get_cell""#
            && e.to == r#"name(& format! ("get_cell"))"#
    }));
}

#[test]
fn external_crate_decycle_test_bug6() {
    let expansions = run_trace_for_repo("decycle", Some("bug6"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "decycle"
            && e.input == r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#
            && e.to == r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_9874485626140785372
{
    ("decycle" "0.3.0" [$_ : path, $wl1 : path $ (,$wl : path) * $ (,) ?]
    { $ ($trait_defs : tt) * } $ ($t : tt) *) =>
    {
        $wl1!
        {
            "decycle" "0.3.0" [$wl1 $ (,$wl) *]
            {
                pub trait LocalTrait { fn local_method(& self) -> usize; }, $
                ($trait_defs) *
            } $ ($t) *
        }
    };
} #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub use
__LocalTrait_temporal_9874485626140785372 as LocalTrait;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "name"
            && e.input == r#""get_cell""#
            && e.to == r#"name(& format! ("get_cell"))"#
    }));
}

#[test]
fn external_crate_decycle_test_coinduction_integration_test() {
    let expansions = run_trace_for_repo("decycle", Some("coinduction_integration_test"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "decycle"
            && e.input == r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#
            && e.to == r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_9874485626140785372
{
    ("decycle" "0.3.0" [$_ : path, $wl1 : path $ (,$wl : path) * $ (,) ?]
    { $ ($trait_defs : tt) * } $ ($t : tt) *) =>
    {
        $wl1!
        {
            "decycle" "0.3.0" [$wl1 $ (,$wl) *]
            {
                pub trait LocalTrait { fn local_method(& self) -> usize; }, $
                ($trait_defs) *
            } $ ($t) *
        }
    };
} #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub use
__LocalTrait_temporal_9874485626140785372 as LocalTrait;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__LocalTrait_temporal_9874485626140785372"
            && e.input == r#""decycle" "0.3.0" [LocalTrait, :: decycle :: __finalize]
          { pub trait TestTrait { fn test_method(& self) -> String; } }
          {
              impl TestTrait for NodeA where NodeB : LocalTrait,
              {
                  fn test_method(& self) -> String
                  {
                      let child_count =
                      self.child_b.as_ref().map_or(0, | b | b.local_method()); format!
                      ("NodeA:{}:{}", self.name, child_count)
                  }
              }, impl LocalTrait for NodeB where NodeA : TestTrait,
              {
                  fn local_method(& self) -> usize
                  {
                      let child_len =
                      self.child_a.as_ref().map_or(0, | a | a.test_method().len());
                      self.count + child_len
                  }
              }
          } 10usize true"#
            && e.to == r#":: decycle :: __finalize !
          {
              "decycle" "0.3.0" [:: decycle :: __finalize]
              {
                  pub trait LocalTrait { fn local_method(& self) -> usize; }, pub trait
                  TestTrait { fn test_method(& self) -> String; }
              }
              {
                  impl TestTrait for NodeA where NodeB : LocalTrait,
                  {
                      fn test_method(& self) -> String
                      {
                          let child_count =
                          self.child_b.as_ref().map_or(0, | b | b.local_method());
                          format! ("NodeA:{}:{}", self.name, child_count)
                      }
                  }, impl LocalTrait for NodeB where NodeA : TestTrait,
                  {
                      fn local_method(& self) -> usize
                      {
                          let child_len =
                          self.child_a.as_ref().map_or(0, | a | a.test_method().len());
                          self.count + child_len
                      }
                  }
              } 10usize true
          }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__finalize"
            && e.input == r#""decycle" "0.3.0" [:: decycle :: __finalize]
{
    pub trait LocalTrait { fn local_method(& self) -> usize; }, pub trait
    TestTrait { fn test_method(& self) -> String; }
}
{
    impl TestTrait for NodeA where NodeB : LocalTrait,
    {
        fn test_method(& self) -> String
        {
            let child_count =
            self.child_b.as_ref().map_or(0, | b | b.local_method()); format!
            ("NodeA:{}:{}", self.name, child_count)
        }
    }, impl LocalTrait for NodeB where NodeA : TestTrait,
    {
        fn local_method(& self) -> usize
        {
            let child_len =
            self.child_a.as_ref().map_or(0, | a | a.test_method().len());
            self.count + child_len
        }
    }
} 10usize true"#
            && e.to == r#"#[doc(hidden)] mod shadowing_module9874485626140785372
{
    trait GetVTableKey9874485626140785372
    {
        extern "C" fn get_vtable_key9874485626140785372(& self) {} fn
        get_cell9874485626140785372(id : :: core :: primitive :: usize) -> &
        'static :: std :: sync :: OnceLock <:: core :: primitive :: usize >
        {
            use :: std :: sync :: { Mutex, OnceLock }; use :: std ::
            collections :: HashMap; use :: std :: primitive ::*; static
            VTABLE_MAP9874485626140785372 : OnceLock < Mutex < HashMap <
            (usize, usize), OnceLock < usize >>>> = OnceLock :: new(); let map
            =
            VTABLE_MAP9874485626140785372.get_or_init(|| Mutex ::
            new(HashMap :: new())); let mut map = map.lock().unwrap(); let r =
            map.entry((Self ::get_vtable_key9874485626140785372 as usize,
            id)).or_insert(OnceLock :: new()); unsafe
            { :: core :: mem :: transmute(r) }
        }
    } impl < T : ?:: core :: marker :: Sized > GetVTableKey9874485626140785372
    for T {} pub mod ranked_traits9874485626140785372
    {
        #[allow(unused)] use super :: super ::*; #[allow(unused)] use super
        ::GetVTableKey9874485626140785372; #[allow(unused)] #[doc(hidden)] pub
        trait LocalTraitRanked9874485626140785372 < Rank9874485626140785372 >
        { fn local_method(& self) -> usize; } #[allow(unused_variables)] impl
        LocalTraitRanked9874485626140785372 < () > for NodeB
        {
            fn local_method(& self) -> usize
            {
                #[allow(unused_unsafe)] unsafe
                {
                    :: core :: mem :: transmute ::< _, fn(& Self) -> usize >
                    (Self ::get_cell9874485626140785372(0usize).get().unwrap())
                    (self)
                }
            }
        } #[allow(unused)] #[doc(hidden)] pub trait
        TestTraitRanked9874485626140785372 < Rank9874485626140785372 >
        { fn test_method(& self) -> String; } #[allow(unused_variables)] impl
        TestTraitRanked9874485626140785372 < () > for NodeA
        {
            fn test_method(& self) -> String
            {
                #[allow(unused_unsafe)] unsafe
                {
                    :: core :: mem :: transmute ::< _, fn(& Self) -> String >
                    (Self ::get_cell9874485626140785372(0usize).get().unwrap())
                    (self)
                }
            }
        }
    } #[allow(unused)] use super ::*; #[allow(non_camel_case_types)] trait
    LocalTrait {} #[allow(non_camel_case_types)] trait TestTrait {}
    #[allow(unused)] use ranked_traits9874485626140785372 ::*;
    #[allow(unused)] use super :: super ::*;
    #[allow(unused_variables, unused_unsafe)] impl < Rank9874485626140785372 >
    ranked_traits9874485626140785372 :: LocalTraitRanked9874485626140785372 <
    (Rank9874485626140785372,) > for NodeB where NodeA :
    ranked_traits9874485626140785372 :: TestTraitRanked9874485626140785372 <
    Rank9874485626140785372 > , Self : ranked_traits9874485626140785372 ::
    LocalTraitRanked9874485626140785372 < Rank9874485626140785372 >
    {
        fn local_method(& self) -> usize
        {
            let _ = Self ::
            get_cell9874485626140785372(0usize).set(< Self as
            ranked_traits9874485626140785372 ::
            LocalTraitRanked9874485626140785372 < Rank9874485626140785372 > >
            :: local_method as _);
            {
                let child_len =
                self.child_a.as_ref().map_or(0, | a | a.test_method().len());
                self.count + child_len
            }
        }
    } #[allow(unused)] use super :: super ::*;
    #[allow(unused_variables, unused_unsafe)] impl < Rank9874485626140785372 >
    ranked_traits9874485626140785372 :: TestTraitRanked9874485626140785372 <
    (Rank9874485626140785372,) > for NodeA where NodeB :
    ranked_traits9874485626140785372 :: LocalTraitRanked9874485626140785372 <
    Rank9874485626140785372 > , Self : ranked_traits9874485626140785372 ::
    TestTraitRanked9874485626140785372 < Rank9874485626140785372 >
    {
        fn test_method(& self) -> String
        {
            let _ = Self ::
            get_cell9874485626140785372(0usize).set(< Self as
            ranked_traits9874485626140785372 ::
            TestTraitRanked9874485626140785372 < Rank9874485626140785372 > >
            :: test_method as _);
            {
                let child_count =
                self.child_b.as_ref().map_or(0, | b | b.local_method());
                format! ("NodeA:{}:{}", self.name, child_count)
            }
        }
    }
} impl LocalTrait for NodeB where Self : shadowing_module9874485626140785372
:: ranked_traits9874485626140785372 :: LocalTraitRanked9874485626140785372 <
(((((((((((),),),),),),),),),),) >
{
    fn local_method(& self) -> usize
    {
        < Self as shadowing_module9874485626140785372
        ::ranked_traits9874485626140785372
        ::LocalTraitRanked9874485626140785372 <
        (((((((((((),),),),),),),),),),) > > ::local_method(self)
    }
} impl TestTrait for NodeA where Self : shadowing_module9874485626140785372 ::
ranked_traits9874485626140785372 :: TestTraitRanked9874485626140785372 <
(((((((((((),),),),),),),),),),) >
{
    fn test_method(& self) -> String
    {
        < Self as shadowing_module9874485626140785372
        ::ranked_traits9874485626140785372
        ::TestTraitRanked9874485626140785372 <
        (((((((((((),),),),),),),),),),) > > ::test_method(self)
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "name"
            && e.input == r#""get_cell""#
            && e.to == r#"name(& format! ("get_cell"))"#
    }));
}

#[test]
fn external_crate_decycle_test_complex() {
    let expansions = run_trace_for_repo("decycle", Some("complex"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "decycle"
            && e.input == r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#
            && e.to == r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_9874485626140785372
{
    ("decycle" "0.3.0" [$_ : path, $wl1 : path $ (,$wl : path) * $ (,) ?]
    { $ ($trait_defs : tt) * } $ ($t : tt) *) =>
    {
        $wl1!
        {
            "decycle" "0.3.0" [$wl1 $ (,$wl) *]
            {
                pub trait LocalTrait { fn local_method(& self) -> usize; }, $
                ($trait_defs) *
            } $ ($t) *
        }
    };
} #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub use
__LocalTrait_temporal_9874485626140785372 as LocalTrait;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "name"
            && e.input == r#""get_cell""#
            && e.to == r#"name(& format! ("get_cell"))"#
    }));
}

#[test]
fn external_crate_decycle_test_complex_coinduction() {
    let expansions = run_trace_for_repo("decycle", Some("complex_coinduction"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "decycle"
            && e.input == r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#
            && e.to == r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_9874485626140785372
{
    ("decycle" "0.3.0" [$_ : path, $wl1 : path $ (,$wl : path) * $ (,) ?]
    { $ ($trait_defs : tt) * } $ ($t : tt) *) =>
    {
        $wl1!
        {
            "decycle" "0.3.0" [$wl1 $ (,$wl) *]
            {
                pub trait LocalTrait { fn local_method(& self) -> usize; }, $
                ($trait_defs) *
            } $ ($t) *
        }
    };
} #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub use
__LocalTrait_temporal_9874485626140785372 as LocalTrait;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "name"
            && e.input == r#""get_cell""#
            && e.to == r#"name(& format! ("get_cell"))"#
    }));
}

#[test]
fn external_crate_decycle_test_min_calculator() {
    let expansions = run_trace_for_repo("decycle", Some("min_calculator"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "decycle"
            && e.input == r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#
            && e.to == r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_9874485626140785372
{
    ("decycle" "0.3.0" [$_ : path, $wl1 : path $ (,$wl : path) * $ (,) ?]
    { $ ($trait_defs : tt) * } $ ($t : tt) *) =>
    {
        $wl1!
        {
            "decycle" "0.3.0" [$wl1 $ (,$wl) *]
            {
                pub trait LocalTrait { fn local_method(& self) -> usize; }, $
                ($trait_defs) *
            } $ ($t) *
        }
    };
} #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub use
__LocalTrait_temporal_9874485626140785372 as LocalTrait;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "name"
            && e.input == r#""get_cell""#
            && e.to == r#"name(& format! ("get_cell"))"#
    }));
}

#[test]
fn external_crate_decycle_test_more_cycles() {
    let expansions = run_trace_for_repo("decycle", Some("more_cycles"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "decycle"
            && e.input == r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#
            && e.to == r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_9874485626140785372
{
    ("decycle" "0.3.0" [$_ : path, $wl1 : path $ (,$wl : path) * $ (,) ?]
    { $ ($trait_defs : tt) * } $ ($t : tt) *) =>
    {
        $wl1!
        {
            "decycle" "0.3.0" [$wl1 $ (,$wl) *]
            {
                pub trait LocalTrait { fn local_method(& self) -> usize; }, $
                ($trait_defs) *
            } $ ($t) *
        }
    };
} #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub use
__LocalTrait_temporal_9874485626140785372 as LocalTrait;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "name"
            && e.input == r#""get_cell""#
            && e.to == r#"name(& format! ("get_cell"))"#
    }));
}

#[test]
fn external_crate_decycle_test_trybuild() {
    let expansions = run_trace_for_repo("decycle", Some("trybuild"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "decycle"
            && e.input == r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#
            && e.to == r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_9874485626140785372
{
    ("decycle" "0.3.0" [$_ : path, $wl1 : path $ (,$wl : path) * $ (,) ?]
    { $ ($trait_defs : tt) * } $ ($t : tt) *) =>
    {
        $wl1!
        {
            "decycle" "0.3.0" [$wl1 $ (,$wl) *]
            {
                pub trait LocalTrait { fn local_method(& self) -> usize; }, $
                ($trait_defs) *
            } $ ($t) *
        }
    };
} #[doc(hidden)] #[allow(unused_imports, unused_macros, dead_code)] pub use
__LocalTrait_temporal_9874485626140785372 as LocalTrait;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "name"
            && e.input == r#""get_cell""#
            && e.to == r#"name(& format! ("get_cell"))"#
    }));
}

#[test]
fn external_crate_addr_of_enum_show_expansion() {
    let expansions = run_trace_for_repo("addr_of_enum", None);
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "chars"
            && e.input == r#"_A _B _C _D _E _F _G _H _I _J _K _L _M _N _O _P _Q _R _S _T _U _V _W _X _Y _Z
           _a _b _c _d _e _f _g _h _i _j _k _l _m _n _o _p _q _r _s _t _u _v _w _x _y _z
           _0 _1 _2 _3 _4 _5 _6 _7 _8 _9 __"#
            && e.to == r#"#[allow(non_camel_case_types)] pub struct _A
           (:: core :: convert :: Infallible); chars!
           (_B _C _D _E _F _G _H _I _J _K _L _M _N _O _P _Q _R _S _T _U _V _W _X _Y _Z _a
           _b _c _d _e _f _g _h _i _j _k _l _m _n _o _p _q _r _s _t _u _v _w _x _y _z _0
           _1 _2 _3 _4 _5 _6 _7 _8 _9 __);"#
    }));
}

#[test]
fn external_crate_addr_of_enum_test_test() {
    let expansions = run_trace_for_repo("addr_of_enum", Some("test"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "addr_of_enum"
            && e.input == r#"&e1, E1, 0"#
            && e.to == r#"< _ as $crate :: EnumHasTagAndField < $crate :: macro_def :: get_tstr!
           ($crate, E1), $crate :: macro_def :: get_tstr! ($crate, 0), >> ::
           addr_of(&e1 as * const _)"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "chars"
            && e.input == r#"_A _B _C _D _E _F _G _H _I _J _K _L _M _N _O _P _Q _R _S _T _U _V _W _X _Y _Z
           _a _b _c _d _e _f _g _h _i _j _k _l _m _n _o _p _q _r _s _t _u _v _w _x _y _z
           _0 _1 _2 _3 _4 _5 _6 _7 _8 _9 __"#
            && e.to == r#"#[allow(non_camel_case_types)] pub struct _A
           (:: core :: convert :: Infallible); chars!
           (_B _C _D _E _F _G _H _I _J _K _L _M _N _O _P _Q _R _S _T _U _V _W _X _Y _Z _a
           _b _c _d _e _f _g _h _i _j _k _l _m _n _o _p _q _r _s _t _u _v _w _x _y _z _0
           _1 _2 _3 _4 _5 _6 _7 _8 _9 __);"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "get_discriminant"
            && e.input == r#"E<u8>, E1"#
            && e.to == r#"< E<u8> as $crate :: EnumHasTag < $crate :: macro_def :: get_tstr!
           ($crate, E1)>> :: discriminant()"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "get_tstr"
            && e.input == r#"$crate, E1"#
            && e.to == r#"($crate :: _tstr :: _E, $crate :: _tstr :: _1)"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Derive
            && e.name == "AddrOfEnum"
            && e.input == r#"#[repr(C)] #[derive(PartialEq, Eq)] enum E<T>
{ E1(usize, u8, u16), E2 { item1: u32, item2: T, }, #[allow(unused)] E3, }"#
            && e.to == r#"const _ : () =
{
    #[automatically_derived] unsafe impl < T > :: addr_of_enum :: AddrOfEnum
    for E < T > {} unsafe impl < T > :: addr_of_enum :: EnumHasTag <
    (:: addr_of_enum :: _tstr :: _E, :: addr_of_enum :: _tstr :: _1), > for E
    < T >
    {
        fn discriminant() -> core :: mem :: Discriminant < Self >
        {
            let val : GhostEnum < T > = GhostEnum
            ::E1(:: core :: mem :: MaybeUninit :: uninit(), :: core :: mem ::
            MaybeUninit :: uninit(), :: core :: mem :: MaybeUninit ::
            uninit(),); #[doc = " SAFETY: both has same memory layout"] unsafe
            {
                :: core :: mem ::
                transmute(:: core :: mem :: discriminant(& val))
            }
        }
    } #[automatically_derived] unsafe impl < T > :: addr_of_enum ::
    EnumHasTagAndField <
    (:: addr_of_enum :: _tstr :: _E, :: addr_of_enum :: _tstr :: _1),
    (:: addr_of_enum :: _tstr :: _0), > for E < T >
    {
        type Ty = usize; fn addr_of(ptr : * const Self) -> * const Self :: Ty
        {
            let en : GhostEnum < T > = GhostEnum
            ::E1(:: core :: mem :: MaybeUninit :: uninit(), :: core :: mem ::
            MaybeUninit :: uninit(), :: core :: mem :: MaybeUninit ::
            uninit(),); match & en
            {
                GhostEnum ::E1(var, _, _,) => unsafe
                {
                    ptr.cast ::< u8 >
                    ().offset((var as * const :: core :: mem :: MaybeUninit <
                    usize > as isize) -
                    (& en as * const GhostEnum < T > as isize)).cast()
                } _ => unsafe { :: core :: hint :: unreachable_unchecked() }
            }
        }
    } #[automatically_derived] unsafe impl < T > :: addr_of_enum ::
    EnumHasTagAndField <
    (:: addr_of_enum :: _tstr :: _E, :: addr_of_enum :: _tstr :: _1),
    (:: addr_of_enum :: _tstr :: _1), > for E < T >
    {
        type Ty = u8; fn addr_of(ptr : * const Self) -> * const Self :: Ty
        {
            let en : GhostEnum < T > = GhostEnum
            ::E1(:: core :: mem :: MaybeUninit :: uninit(), :: core :: mem ::
            MaybeUninit :: uninit(), :: core :: mem :: MaybeUninit ::
            uninit(),); match & en
            {
                GhostEnum ::E1(_, var, _,) => unsafe
                {
                    ptr.cast ::< u8 >
                    ().offset((var as * const :: core :: mem :: MaybeUninit < u8
                    > as isize) -
                    (& en as * const GhostEnum < T > as isize)).cast()
                } _ => unsafe { :: core :: hint :: unreachable_unchecked() }
            }
        }
    } #[automatically_derived] unsafe impl < T > :: addr_of_enum ::
    EnumHasTagAndField <
    (:: addr_of_enum :: _tstr :: _E, :: addr_of_enum :: _tstr :: _1),
    (:: addr_of_enum :: _tstr :: _2), > for E < T >
    {
        type Ty = u16; fn addr_of(ptr : * const Self) -> * const Self :: Ty
        {
            let en : GhostEnum < T > = GhostEnum
            ::E1(:: core :: mem :: MaybeUninit :: uninit(), :: core :: mem ::
            MaybeUninit :: uninit(), :: core :: mem :: MaybeUninit ::
            uninit(),); match & en
            {
                GhostEnum ::E1(_, _, var,) => unsafe
                {
                    ptr.cast ::< u8 >
                    ().offset((var as * const :: core :: mem :: MaybeUninit <
                    u16 > as isize) -
                    (& en as * const GhostEnum < T > as isize)).cast()
                } _ => unsafe { :: core :: hint :: unreachable_unchecked() }
            }
        }
    } unsafe impl < T > :: addr_of_enum :: EnumHasTag <
    (:: addr_of_enum :: _tstr :: _E, :: addr_of_enum :: _tstr :: _2), > for E
    < T >
    {
        fn discriminant() -> core :: mem :: Discriminant < Self >
        {
            let val : GhostEnum < T > = GhostEnum ::E2
            {
                item1 : :: core :: mem :: MaybeUninit :: uninit(), item2 : ::
                core :: mem :: MaybeUninit :: uninit(),
            }; #[doc = " SAFETY: both has same memory layout"] unsafe
            {
                :: core :: mem ::
                transmute(:: core :: mem :: discriminant(& val))
            }
        }
    } #[automatically_derived] unsafe impl < T > :: addr_of_enum ::
    EnumHasTagAndField <
    (:: addr_of_enum :: _tstr :: _E, :: addr_of_enum :: _tstr :: _2),
    (:: addr_of_enum :: _tstr :: _i, :: addr_of_enum :: _tstr :: _t, ::
    addr_of_enum :: _tstr :: _e, :: addr_of_enum :: _tstr :: _m, ::
    addr_of_enum :: _tstr :: _1), > for E < T >
    {
        type Ty = u32; fn addr_of(ptr : * const Self) -> * const Self :: Ty
        {
            let en : GhostEnum < T > = GhostEnum ::E2
            {
                item1 : :: core :: mem :: MaybeUninit :: uninit(), item2 : ::
                core :: mem :: MaybeUninit :: uninit(),
            }; match & en
            {
                GhostEnum ::E2 { item1, .. } => unsafe
                {
                    ptr.cast ::< u8 >
                    ().offset((item1 as * const :: core :: mem :: MaybeUninit <
                    u32 > as isize) -
                    (& en as * const GhostEnum < T > as isize)).cast()
                } _ => unsafe { :: core :: hint :: unreachable_unchecked() }
            }
        }
    } #[automatically_derived] unsafe impl < T > :: addr_of_enum ::
    EnumHasTagAndField <
    (:: addr_of_enum :: _tstr :: _E, :: addr_of_enum :: _tstr :: _2),
    (:: addr_of_enum :: _tstr :: _i, :: addr_of_enum :: _tstr :: _t, ::
    addr_of_enum :: _tstr :: _e, :: addr_of_enum :: _tstr :: _m, ::
    addr_of_enum :: _tstr :: _2), > for E < T >
    {
        type Ty = T; fn addr_of(ptr : * const Self) -> * const Self :: Ty
        {
            let en : GhostEnum < T > = GhostEnum ::E2
            {
                item1 : :: core :: mem :: MaybeUninit :: uninit(), item2 : ::
                core :: mem :: MaybeUninit :: uninit(),
            }; match & en
            {
                GhostEnum ::E2 { item2, .. } => unsafe
                {
                    ptr.cast ::< u8 >
                    ().offset((item2 as * const :: core :: mem :: MaybeUninit <
                    T > as isize) -
                    (& en as * const GhostEnum < T > as isize)).cast()
                } _ => unsafe { :: core :: hint :: unreachable_unchecked() }
            }
        }
    } unsafe impl < T > :: addr_of_enum :: EnumHasTag <
    (:: addr_of_enum :: _tstr :: _E, :: addr_of_enum :: _tstr :: _3), > for E
    < T >
    {
        fn discriminant() -> core :: mem :: Discriminant < Self >
        {
            let val : GhostEnum < T > = GhostEnum ::E3;
            #[doc = " SAFETY: both has same memory layout"] unsafe
            {
                :: core :: mem ::
                transmute(:: core :: mem :: discriminant(& val))
            }
        }
    } #[repr(C)] enum GhostEnum < T >
    {
        E1(:: core :: mem :: MaybeUninit < usize > , :: core :: mem ::
        MaybeUninit < u8 > , :: core :: mem :: MaybeUninit < u16 >), E2
        {
            item1 : :: core :: mem :: MaybeUninit < u32 > , item2 : :: core ::
            mem :: MaybeUninit < T > ,
        }, #[allow(unused)] E3,
    }
};"#
    }));
}

#[test]
fn external_crate_discriminant_show_expansion() {
    let expansions = run_trace_for_repo("discriminant", None);
}

#[test]
fn external_crate_discriminant_test_test() {
    let expansions = run_trace_for_repo("discriminant", Some("test"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Derive
            && e.name == "Enum"
            && e.input == r#"#[allow(unused)] #[repr(u8)] pub enum MixedEnum<T>
{
    UnitVariantA = 1, TupleVariantB(i32, f64), StructVariantC
    { name: String, value: T }, SomeValue(T), NoneValue = 99,
    TupleWithGeneric(T, usize),
}"#
            && e.to == r#"#[repr(u8)]
#[derive(:: core :: marker :: Copy, :: core :: clone :: Clone, :: core :: fmt
:: Debug, :: core :: hash :: Hash, :: core :: cmp :: PartialEq, :: core :: cmp
:: Eq,)] pub enum __Discriminant_MixedEnum_528
{
    UnitVariantA = 1, TupleVariantB, StructVariantC, SomeValue, NoneValue =
    99, TupleWithGeneric,
} impl :: core :: fmt :: Display for __Discriminant_MixedEnum_528
{
    fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
    fmt :: Result { < Self as :: core :: fmt :: Debug >:: fmt(self, f) }
} impl :: core :: cmp :: PartialOrd for __Discriminant_MixedEnum_528
{
    fn partial_cmp(& self, other : & Self) -> :: core :: option :: Option <::
    core :: cmp :: Ordering >
    { (* self as u8).partial_cmp(& (* other as u8)) }
} impl :: core :: cmp :: Ord for __Discriminant_MixedEnum_528
{
    fn cmp(& self, other : & Self) -> :: core :: cmp :: Ordering
    { (* self as u8).cmp(& (* other as u8)) }
} #[automatically_derived] unsafe impl < T > :: discriminant :: Enum for
MixedEnum < T >
{
    type Discriminant = __Discriminant_MixedEnum_528; fn discriminant(& self)
    -> Self :: Discriminant
    {
        match self
        {
            Self ::UnitVariantA => __Discriminant_MixedEnum_528
            ::UnitVariantA, Self ::TupleVariantB(..) =>
            __Discriminant_MixedEnum_528 ::TupleVariantB, Self
            ::StructVariantC { .. } => __Discriminant_MixedEnum_528
            ::StructVariantC, Self ::SomeValue(..) =>
            __Discriminant_MixedEnum_528 ::SomeValue, Self ::NoneValue =>
            __Discriminant_MixedEnum_528 ::NoneValue, Self
            ::TupleWithGeneric(..) => __Discriminant_MixedEnum_528
            ::TupleWithGeneric,
        }
    }
} impl :: core :: convert :: TryFrom <u8 > for __Discriminant_MixedEnum_528
{
    type Error = (); fn try_from(value : u8) -> :: core :: result :: Result <
    Self, Self :: Error >
    {
        if value == 1
        { :: core :: result :: Result :: Ok(Self ::UnitVariantA) } else if
        value == 1 + 1
        { :: core :: result :: Result :: Ok(Self ::TupleVariantB) } else if
        value == 1 + 1 + 1
        { :: core :: result :: Result :: Ok(Self ::StructVariantC) } else if
        value == 1 + 1 + 1 + 1
        { :: core :: result :: Result :: Ok(Self ::SomeValue) } else if value
        == 99 { :: core :: result :: Result :: Ok(Self ::NoneValue) } else if
        value == 99 + 1
        { :: core :: result :: Result :: Ok(Self ::TupleWithGeneric) } else
        { :: core :: result :: Result :: Err(()) }
    }
} impl :: core :: convert :: Into <u8 > for __Discriminant_MixedEnum_528
{ fn into(self) -> u8 { self as u8 } } unsafe impl :: discriminant ::
Discriminant for __Discriminant_MixedEnum_528
{
    type Repr = u8; fn all() -> impl :: core :: iter :: Iterator < Item = Self
    >
    {
        struct
        Iter(:: core :: option :: Option <__Discriminant_MixedEnum_528 >);
        impl :: core :: iter :: Iterator for Iter
        {
            type Item = __Discriminant_MixedEnum_528; fn next(& mut self) ->
            Option < Self :: Item >
            {
                match self.0
                {
                    :: core :: option :: Option ::
                    Some(__Discriminant_MixedEnum_528 ::UnitVariantA) =>
                    {
                        let ret = self.0; self.0 =
                        Some(__Discriminant_MixedEnum_528 ::TupleVariantB); ret
                    } :: core :: option :: Option ::
                    Some(__Discriminant_MixedEnum_528 ::TupleVariantB) =>
                    {
                        let ret = self.0; self.0 =
                        Some(__Discriminant_MixedEnum_528 ::StructVariantC); ret
                    } :: core :: option :: Option ::
                    Some(__Discriminant_MixedEnum_528 ::StructVariantC) =>
                    {
                        let ret = self.0; self.0 =
                        Some(__Discriminant_MixedEnum_528 ::SomeValue); ret
                    } :: core :: option :: Option ::
                    Some(__Discriminant_MixedEnum_528 ::SomeValue) =>
                    {
                        let ret = self.0; self.0 =
                        Some(__Discriminant_MixedEnum_528 ::NoneValue); ret
                    } :: core :: option :: Option ::
                    Some(__Discriminant_MixedEnum_528 ::NoneValue) =>
                    {
                        let ret = self.0; self.0 =
                        Some(__Discriminant_MixedEnum_528 ::TupleWithGeneric); ret
                    } :: core :: option :: Option ::
                    Some(__Discriminant_MixedEnum_528 ::TupleWithGeneric) =>
                    { let ret = self.0; self.0 = None; ret } :: core :: option
                    :: Option :: None => :: core :: option :: Option :: None,
                }
            } fn size_hint(& self) ->
            (:: core :: primitive :: usize, :: core :: option :: Option <::
            core :: primitive :: usize >)
            {
                let n = Self(self.0).count();
                (n, :: core :: option :: Option :: Some(n))
            } fn count(self) -> usize
            {
                match self.0
                {
                    :: core :: option :: Option ::
                    Some(__Discriminant_MixedEnum_528 ::UnitVariantA) => 6usize,
                    :: core :: option :: Option ::
                    Some(__Discriminant_MixedEnum_528 ::TupleVariantB) =>
                    5usize, :: core :: option :: Option ::
                    Some(__Discriminant_MixedEnum_528 ::StructVariantC) =>
                    4usize, :: core :: option :: Option ::
                    Some(__Discriminant_MixedEnum_528 ::SomeValue) => 3usize, ::
                    core :: option :: Option ::
                    Some(__Discriminant_MixedEnum_528 ::NoneValue) => 2usize, ::
                    core :: option :: Option ::
                    Some(__Discriminant_MixedEnum_528 ::TupleWithGeneric) =>
                    1usize, :: core :: option :: Option :: None => 0,
                }
            } fn last(self) -> Option < Self :: Item >
            {
                self.0.map(| _ | __Discriminant_MixedEnum_528
                ::TupleWithGeneric)
            }
        }
        Iter(:: core :: option :: Option ::
        Some(__Discriminant_MixedEnum_528 ::UnitVariantA))
    }
}"#
    }));
}

#[test]
fn external_crate_flat_enum_show_expansion() {
    let expansions = run_trace_for_repo("flat_enum", None);
}

#[test]
fn external_crate_newer_type_show_expansion() {
    let expansions = run_trace_for_repo("newer-type", None);
}

#[test]
fn external_crate_newer_type_test_enum() {
    let expansions = run_trace_for_repo("newer-type", Some("enum"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "implement"
            && e.input == r#"enum BasicEnum
{
    VariantA(#[implement(BasicEnumTrait)] i32),
    VariantB(#[implement(BasicEnumTrait)] i32),
}"#
            && e.to == r#"enum BasicEnum { VariantA(i32), VariantB(i32), } BasicEnumTrait!
{
    (BasicEnumTrait) enum BasicEnum
    {
        VariantA(#[implement(BasicEnumTrait)] i32),
        VariantB(#[implement(BasicEnumTrait)] i32),
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "target"
            && e.input == r#"trait BasicEnumTrait { fn value(&self) -> i32; }"#
            && e.to == r#"#[doc(hidden)] #[macro_export] macro_rules!
__newer_type_macro__11883582657124695657
{
    ($ ($t : tt) *) =>
    {
        :: newer_type :: __implement_internal!
        {
            ($ ($t) *) trait BasicEnumTrait { fn value(& self) -> i32; }, , ::
            newer_type, (i32), Repeater, 11883582657124695657u64
        }
    }
} #[doc(hidden)] use __newer_type_macro__11883582657124695657 as
BasicEnumTrait; #[allow(private_bounds)]
#[allow(clippy :: missing_safety_doc)] trait BasicEnumTrait
{ fn value(& self) -> i32; } impl < __NewerTypeSelf11883582657124695657 >
Repeater <11883582657124695657u64, 0usize, () > for
__NewerTypeSelf11883582657124695657 where Self : BasicEnumTrait,
{ type Type = i32; }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__implement_internal"
            && e.input == r#"((MultiImplementTrait) enum MultiImplementEnum
{
    VariantOne(#[implement(MultiImplementTrait)] i32),
    VariantTwo(#[implement(MultiImplementTrait)] i32, i32),
}) trait MultiImplementTrait { fn double(& self) -> i32; }, , :: newer_type,
(i32), Repeater, 3688368205373548515u64"#
            && e.to == r#"#[automatically_derived] impl < > MultiImplementTrait for MultiImplementEnum
where i32 : MultiImplementTrait <> , i32 : MultiImplementTrait <>
{
    fn double(& self) -> < Self as Repeater < 3688368205373548515u64, 0usize,
    () > > :: Type
    {
        match self
        {
            Self ::VariantOne(__newer_type_pred_param) =>
            {
                < _ as MultiImplementTrait > ::
                double(__newer_type_pred_param)
            } Self ::VariantTwo(__newer_type_pred_param, _) =>
            {
                < _ as MultiImplementTrait > ::
                double(__newer_type_pred_param)
            }
        }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__3688368205373548515"
            && e.input == r#"(MultiImplementTrait) enum MultiImplementEnum
            {
                VariantOne(#[implement(MultiImplementTrait)] i32),
                VariantTwo(#[implement(MultiImplementTrait)] i32, i32),
            }"#
            && e.to == r#":: newer_type :: __implement_internal!
            {
                ((MultiImplementTrait) enum MultiImplementEnum
                {
                    VariantOne(#[implement(MultiImplementTrait)] i32),
                    VariantTwo(#[implement(MultiImplementTrait)] i32, i32),
                }) trait MultiImplementTrait { fn double(& self) -> i32; }, , ::
                newer_type, (i32), Repeater, 3688368205373548515u64
            }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__2202241493200732658"
            && e.input == r#"(NestedEnumTrait) enum NestedEnum
            { Variant(#[implement(NestedEnumTrait)] Box < i32 >), }"#
            && e.to == r#":: newer_type :: __implement_internal!
            {
                ((NestedEnumTrait) enum NestedEnum
                { Variant(#[implement(NestedEnumTrait)] Box < i32 >), }) trait
                NestedEnumTrait { fn nested_value(& self) -> i32; }, , :: newer_type,
                (i32), Repeater, 2202241493200732658u64
            }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__17308691450604305863"
            && e.input == r#"(ComplexEnumTrait) enum ComplexEnum
            {
                Named { id : u32, #[implement(ComplexEnumTrait)] data : (i32, i32), },
                Tuple(u32, #[implement(ComplexEnumTrait)] (i32, i32)),
            }"#
            && e.to == r#":: newer_type :: __implement_internal!
            {
                ((ComplexEnumTrait) enum ComplexEnum
                {
                    Named { id : u32, #[implement(ComplexEnumTrait)] data : (i32, i32), },
                    Tuple(u32, #[implement(ComplexEnumTrait)] (i32, i32)),
                }) trait ComplexEnumTrait { fn compute(& self) -> i32; }, , :: newer_type,
                (i32), Repeater, 17308691450604305863u64
            }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__18041360355863048294"
            && e.input == r#"(GenericEnumTrait < U >) enum GenericEnum < U : Clone + Debug >
           {
               First(#[implement(GenericEnumTrait<U>)] U),
               Second(#[implement(GenericEnumTrait<U>)] U),
           }"#
            && e.to == r#":: newer_type :: __implement_internal!
           {
               ((GenericEnumTrait < U >) enum GenericEnum < U : Clone + Debug >
               {
                   First(#[implement(GenericEnumTrait<U>)] U),
                   Second(#[implement(GenericEnumTrait<U>)] U),
               }) trait GenericEnumTrait < T > { fn describe(& self) -> String; }, , ::
               newer_type, (String), Repeater, 18041360355863048294u64
           }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__5406477267822067746"
            && e.input == r#"(NamedEnumTrait) enum NamedEnum
           {
               Named { a : i32, #[implement(NamedEnumTrait)] b : i32, },
               Tuple(#[implement(NamedEnumTrait)] i32, i32),
           }"#
            && e.to == r#":: newer_type :: __implement_internal!
           {
               ((NamedEnumTrait) enum NamedEnum
               {
                   Named { a : i32, #[implement(NamedEnumTrait)] b : i32, },
                   Tuple(#[implement(NamedEnumTrait)] i32, i32),
               }) trait NamedEnumTrait { fn sum(& self) -> i32; }, , :: newer_type,
               (i32), Repeater, 5406477267822067746u64
           }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__11883582657124695657"
            && e.input == r#"(BasicEnumTrait) enum BasicEnum
           {
               VariantA(#[implement(BasicEnumTrait)] i32),
               VariantB(#[implement(BasicEnumTrait)] i32),
           }"#
            && e.to == r#":: newer_type :: __implement_internal!
           {
               ((BasicEnumTrait) enum BasicEnum
               {
                   VariantA(#[implement(BasicEnumTrait)] i32),
                   VariantB(#[implement(BasicEnumTrait)] i32),
               }) trait BasicEnumTrait { fn value(& self) -> i32; }, , :: newer_type,
               (i32), Repeater, 11883582657124695657u64
           }"#
    }));
}

#[test]
fn external_crate_newer_type_test_multi_self() {
    let expansions = run_trace_for_repo("newer-type", Some("multi_self"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "implement"
            && e.input == r#"struct MultiSelfArgNewType(TestType);"#
            && e.to == r#"struct MultiSelfArgNewType(TestType); MultiSelfArgTrait!
{ (MultiSelfArgTrait) struct MultiSelfArgNewType(TestType); }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "target"
            && e.input == r#"trait MultiSelfArgTrait
{
    fn process(self, other: Self, reference: &Self, mutable: &mut Self) ->
    i32; fn process_no_receiver(other: Self, reference: &Self) -> bool; fn
    process_with_ref(&self, other: &Self) -> String; fn
    process_with_mut(&mut self, other: &mut Self);
}"#
            && e.to == r#"#[doc(hidden)] #[macro_export] macro_rules!
__newer_type_macro__18272456070772774426
{
    ($ ($t : tt) *) =>
    {
        :: newer_type :: __implement_internal!
        {
            ($ ($t) *) trait MultiSelfArgTrait
            {
                fn
                process(self, other : Self, reference : & Self, mutable : &
                mut Self) -> i32; fn
                process_no_receiver(other : Self, reference : & Self) -> bool;
                fn process_with_ref(& self, other : & Self) -> String; fn
                process_with_mut(& mut self, other : & mut Self);
            }, , :: newer_type, (i32, String, bool), Repeater,
            18272456070772774426u64
        }
    }
} #[doc(hidden)] use __newer_type_macro__18272456070772774426 as
MultiSelfArgTrait; #[allow(private_bounds)]
#[allow(clippy :: missing_safety_doc)] trait MultiSelfArgTrait
{
    fn process(self, other : Self, reference : & Self, mutable : & mut Self)
    -> i32; fn process_no_receiver(other : Self, reference : & Self) -> bool;
    fn process_with_ref(& self, other : & Self) -> String; fn
    process_with_mut(& mut self, other : & mut Self);
} impl < __NewerTypeSelf18272456070772774426 > Repeater
<18272456070772774426u64, 0usize, () > for __NewerTypeSelf18272456070772774426
where Self : MultiSelfArgTrait, { type Type = i32; } impl <
__NewerTypeSelf18272456070772774426 > Repeater <18272456070772774426u64,
1usize, () > for __NewerTypeSelf18272456070772774426 where Self :
MultiSelfArgTrait, { type Type = String; } impl <
__NewerTypeSelf18272456070772774426 > Repeater <18272456070772774426u64,
2usize, () > for __NewerTypeSelf18272456070772774426 where Self :
MultiSelfArgTrait, { type Type = bool; }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__implement_internal"
            && e.input == r#"((MultiSelfArgTrait) struct DeeplyNestedStruct
{ #[implement(MultiSelfArgTrait)] deep : NestedStruct, _extra_data : i64, })
trait MultiSelfArgTrait
{
    fn process(self, other : Self, reference : & Self, mutable : & mut Self)
    -> i32; fn process_no_receiver(other : Self, reference : & Self) -> bool;
    fn process_with_ref(& self, other : & Self) -> String; fn
    process_with_mut(& mut self, other : & mut Self);
}, , :: newer_type, (i32, String, bool), Repeater, 18272456070772774426u64"#
            && e.to == r#"#[automatically_derived] impl < > MultiSelfArgTrait for DeeplyNestedStruct
where NestedStruct : MultiSelfArgTrait <>
{
    fn process(self, other : Self, reference : & Self, mutable : & mut Self)
    -> < Self as Repeater < 18272456070772774426u64, 0usize, () > > :: Type
    {
        let Self { deep : __newer_type_pred_param_0, .. } = self; let Self
        { deep : __newer_type_pred_param_1, .. } = other; let Self
        { deep : __newer_type_pred_param_2, .. } = reference; let Self
        { deep : __newer_type_pred_param_3, .. } = mutable; < _ as
        MultiSelfArgTrait > ::
        process(__newer_type_pred_param_0, __newer_type_pred_param_1,
        __newer_type_pred_param_2, __newer_type_pred_param_3)
    } fn process_no_receiver(other : Self, reference : & Self) -> < Self as
    Repeater < 18272456070772774426u64, 2usize, () > > :: Type
    {
        let Self { deep : __newer_type_pred_param_0, .. } = other; let Self
        { deep : __newer_type_pred_param_1, .. } = reference; < _ as
        MultiSelfArgTrait > ::
        process_no_receiver(__newer_type_pred_param_0,
        __newer_type_pred_param_1)
    } fn process_with_ref(& self, other : & Self) -> < Self as Repeater <
    18272456070772774426u64, 1usize, () > > :: Type
    {
        let Self { deep : __newer_type_pred_param_0, .. } = self; let Self
        { deep : __newer_type_pred_param_1, .. } = other; < _ as
        MultiSelfArgTrait > ::
        process_with_ref(__newer_type_pred_param_0, __newer_type_pred_param_1)
    } fn process_with_mut(& mut self, other : & mut Self)
    {
        let Self { deep : __newer_type_pred_param_0, .. } = self; let Self
        { deep : __newer_type_pred_param_1, .. } = other; < _ as
        MultiSelfArgTrait > ::
        process_with_mut(__newer_type_pred_param_0, __newer_type_pred_param_1)
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__18272456070772774426"
            && e.input == r#"(MultiSelfArgTrait) struct DeeplyNestedStruct
            { #[implement(MultiSelfArgTrait)] deep : NestedStruct, _extra_data : i64, }"#
            && e.to == r#":: newer_type :: __implement_internal!
            {
                ((MultiSelfArgTrait) struct DeeplyNestedStruct
                {
                    #[implement(MultiSelfArgTrait)] deep : NestedStruct, _extra_data :
                    i64,
                }) trait MultiSelfArgTrait
                {
                    fn
                    process(self, other : Self, reference : & Self, mutable : & mut Self)
                    -> i32; fn process_no_receiver(other : Self, reference : & Self) ->
                    bool; fn process_with_ref(& self, other : & Self) -> String; fn
                    process_with_mut(& mut self, other : & mut Self);
                }, , :: newer_type, (i32, String, bool), Repeater, 18272456070772774426u64
            }"#
    }));
}

#[test]
fn external_crate_newer_type_test_string() {
    let expansions = run_trace_for_repo("newer-type", Some("string"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "implement"
            && e.input == r#"#[allow(unused)] struct MyStruct { slot: u8, }"#
            && e.to == r#"#[allow(unused)] struct MyStruct { slot : u8, } ToString!
{ (ToString) #[allow(unused)] struct MyStruct { slot : u8, } }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "target"
            && e.input == r#"pub trait ToString { fn to_string(&self) -> String; }"#
            && e.to == r#"#[doc(hidden)] #[macro_export] macro_rules!
__newer_type_macro__11709415566274083355
{
    ($ ($t : tt) *) =>
    {
        :: newer_type :: __implement_internal!
        {
            ($ ($t) *) pub trait ToString { fn to_string(& self) -> String; },
            :: std :: string :: ToString, :: newer_type, (String), Repeater,
            11709415566274083355u64
        }
    }
} #[doc(hidden)] pub use __newer_type_macro__11709415566274083355 as ToString;
#[allow(private_bounds)] #[allow(clippy :: missing_safety_doc)]
#[doc = " # Safety"] #[doc = " "]
#[doc = " should be implemented by [`newer_type::implement`]"] pub unsafe
trait ToString : :: std :: string :: ToString < > {} impl <
__NewerTypeSelf11709415566274083355 > Repeater <11709415566274083355u64,
0usize, () > for __NewerTypeSelf11709415566274083355 where Self : ToString,
{ type Type = String; }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__implement_internal"
            && e.input == r#"((ToString) #[allow(unused)] struct MyStruct { slot : u8, }) pub trait
ToString { fn to_string(& self) -> String; }, :: std :: string :: ToString, ::
newer_type, (String), Repeater, 11709415566274083355u64"#
            && e.to == r#"#[automatically_derived] impl < > :: std :: string :: ToString for MyStruct
where u8 : :: std :: string :: ToString <>
{
    fn to_string(& self) -> < Self as Repeater < 11709415566274083355u64,
    0usize, () > > :: Type
    {
        let Self { slot : __newer_type_pred_param_0, .. } = self; < _ as ::
        std :: string :: ToString > :: to_string(__newer_type_pred_param_0)
    }
} #[automatically_derived] unsafe impl < > ToString for MyStruct where u8 : ::
std :: string :: ToString <> {}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__11709415566274083355"
            && e.input == r#"(ToString) #[allow(unused)] struct MyStruct { slot : u8, }"#
            && e.to == r#":: newer_type :: __implement_internal!
           {
               ((ToString) #[allow(unused)] struct MyStruct { slot : u8, }) pub trait
               ToString { fn to_string(& self) -> String; }, :: std :: string ::
               ToString, :: newer_type, (String), Repeater, 11709415566274083355u64
           }"#
    }));
}

#[test]
fn external_crate_newer_type_test_test2() {
    let expansions = run_trace_for_repo("newer-type", Some("test2"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "implement"
            && e.input == r#"struct MyNewType(MyExistingType);"#
            && e.to == r#"struct MyNewType(MyExistingType); MyTrait!
{ (MyTrait) struct MyNewType(MyExistingType); }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "target"
            && e.input == r#"trait MyTrait { fn value(&self) -> i32; }"#
            && e.to == r#"#[doc(hidden)] #[macro_export] macro_rules!
__newer_type_macro__5658914704125183378
{
    ($ ($t : tt) *) =>
    {
        :: newer_type :: __implement_internal!
        {
            ($ ($t) *) trait MyTrait { fn value(& self) -> i32; }, , ::
            newer_type, (i32), Repeater, 5658914704125183378u64
        }
    }
} #[doc(hidden)] use __newer_type_macro__5658914704125183378 as MyTrait;
#[allow(private_bounds)] #[allow(clippy :: missing_safety_doc)] trait MyTrait
{ fn value(& self) -> i32; } impl < __NewerTypeSelf5658914704125183378 >
Repeater <5658914704125183378u64, 0usize, () > for
__NewerTypeSelf5658914704125183378 where Self : MyTrait, { type Type = i32; }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__implement_internal"
            && e.input == r#"((DefaultTrait) struct DefaultNewType(MyExistingType);) trait DefaultTrait
{ fn default_value(& self) -> i32 { 999 } }, , :: newer_type, (i32), Repeater,
11289885470094395680u64"#
            && e.to == r#"#[automatically_derived] impl < > DefaultTrait for DefaultNewType where
MyExistingType : DefaultTrait <>
{
    fn default_value(& self) -> < Self as Repeater < 11289885470094395680u64,
    0usize, () > > :: Type
    {
        let Self(__newer_type_pred_param_0) = self; < _ as DefaultTrait > ::
        default_value(__newer_type_pred_param_0)
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__11289885470094395680"
            && e.input == r#"(DefaultTrait) struct DefaultNewType(MyExistingType);"#
            && e.to == r#":: newer_type :: __implement_internal!
            {
                ((DefaultTrait) struct DefaultNewType(MyExistingType);) trait DefaultTrait
                { fn default_value(& self) -> i32 { 999 } }, , :: newer_type, (i32),
                Repeater, 11289885470094395680u64
            }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__5658914704125183378"
            && e.input == r#"(MyTrait) struct CopyNewType(MyExistingType);"#
            && e.to == r#":: newer_type :: __implement_internal!
            {
                ((MyTrait) struct CopyNewType(MyExistingType);) trait MyTrait
                { fn value(& self) -> i32; }, , :: newer_type, (i32), Repeater,
                5658914704125183378u64
            }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__3805815411622096210"
            && e.input == r#"(GenericTrait < T >) struct GenericNewType < T > (Option < T >);"#
            && e.to == r#":: newer_type :: __implement_internal!
           {
               ((GenericTrait < T >) struct GenericNewType < T > (Option < T >);) trait
               GenericTrait < T > { fn get_value(& self) -> & T; }, , :: newer_type, (),
               Repeater, 3805815411622096210u64
           }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__12483158918998798112"
            && e.input == r#"(AnotherTrait) struct DualTraitNewType(MyExistingType);"#
            && e.to == r#":: newer_type :: __implement_internal!
           {
               ((AnotherTrait) struct DualTraitNewType(MyExistingType);) trait
               AnotherTrait { fn double_value(& self) -> i32; }, , :: newer_type, (i32),
               Repeater, 12483158918998798112u64
           }"#
    }));
}

#[test]
fn external_crate_newer_type_test_test3() {
    let expansions = run_trace_for_repo("newer-type", Some("test3"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "implement"
            && e.input == r#"struct BasicNewType(BasicType);"#
            && e.to == r#"struct BasicNewType(BasicType); BasicTrait!
{ (BasicTrait) struct BasicNewType(BasicType); }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "target"
            && e.input == r#"trait BasicTrait
{
    fn get_number(&self) -> i32; fn double_number(&self) -> i32
    { self.get_number() * 2 }
}"#
            && e.to == r#"#[doc(hidden)] #[macro_export] macro_rules!
__newer_type_macro__16719574466313965748
{
    ($ ($t : tt) *) =>
    {
        :: newer_type :: __implement_internal!
        {
            ($ ($t) *) trait BasicTrait
            {
                fn get_number(& self) -> i32; fn double_number(& self) -> i32
                { self.get_number() * 2 }
            }, , :: newer_type, (i32), Repeater, 16719574466313965748u64
        }
    }
} #[doc(hidden)] use __newer_type_macro__16719574466313965748 as BasicTrait;
#[allow(private_bounds)] #[allow(clippy :: missing_safety_doc)] trait
BasicTrait
{
    fn get_number(& self) -> i32; fn double_number(& self) -> i32
    { self.get_number() * 2 }
} impl < __NewerTypeSelf16719574466313965748 > Repeater
<16719574466313965748u64, 0usize, () > for __NewerTypeSelf16719574466313965748
where Self : BasicTrait, { type Type = i32; }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__implement_internal"
            && e.input == r#"((AssociatedConstTrait) struct AssociatedConstNewType(BasicType);) trait
AssociatedConstTrait
{ const VALUE : i32; fn get_const_value(& self) -> i32 { Self :: VALUE } }, ,
:: newer_type, (i32), Repeater, 7968502500002432695u64"#
            && e.to == r#"#[automatically_derived] impl < > AssociatedConstTrait for
AssociatedConstNewType where BasicType : AssociatedConstTrait <>
{
    const VALUE : i32 = <BasicType as AssociatedConstTrait >::VALUE; fn
    get_const_value(& self) -> < Self as Repeater < 7968502500002432695u64,
    0usize, () > > :: Type
    {
        let Self(__newer_type_pred_param_0) = self; < _ as
        AssociatedConstTrait > :: get_const_value(__newer_type_pred_param_0)
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__7968502500002432695"
            && e.input == r#"(AssociatedConstTrait) struct AssociatedConstNewType(BasicType);"#
            && e.to == r#":: newer_type :: __implement_internal!
            {
                ((AssociatedConstTrait) struct AssociatedConstNewType(BasicType);) trait
                AssociatedConstTrait
                {
                    const VALUE : i32; fn get_const_value(& self) -> i32 { Self :: VALUE }
                }, , :: newer_type, (i32), Repeater, 7968502500002432695u64
            }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__14667447593113196270"
            && e.input == r#"(ComplexConstraintTrait < String >) struct
            ComplexConstraintNewType(BasicType);"#
            && e.to == r#":: newer_type :: __implement_internal!
            {
                ((ComplexConstraintTrait < String >) struct
                ComplexConstraintNewType(BasicType);) trait ComplexConstraintTrait < T >
                where T : :: core :: fmt :: Debug + :: core :: clone :: Clone + :: core ::
                cmp :: PartialEq + :: core :: default :: Default,
                { fn process_item(& self, item : T) -> T; }, , :: newer_type, (),
                Repeater, 14667447593113196270u64
            }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__14204025672217719572"
            && e.input == r#"(MutatingTrait) struct MutatingNewType(BasicType);"#
            && e.to == r#":: newer_type :: __implement_internal!
            {
                ((MutatingTrait) struct MutatingNewType(BasicType);) trait MutatingTrait
                { fn increment(& mut self); }, , :: newer_type, (), Repeater,
                14204025672217719572u64
            }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__12661960518354390351"
            && e.input == r#"(AssociatedTypeTrait) struct AssociatedTypeNewType(BasicType);"#
            && e.to == r#":: newer_type :: __implement_internal!
            {
                ((AssociatedTypeTrait) struct AssociatedTypeNewType(BasicType);) trait
                AssociatedTypeTrait
                { type Output; fn compute(& self) -> Self :: Output; }, , :: newer_type,
                (), Repeater, 12661960518354390351u64
            }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__13541114746309738512"
            && e.input == r#"(FunctionPointerTrait) struct FunctionPointerNewType(BasicType);"#
            && e.to == r#":: newer_type :: __implement_internal!
            {
                ((FunctionPointerTrait) struct FunctionPointerNewType(BasicType);) trait
                FunctionPointerTrait { fn apply_fn(& self, f : fn(i32) -> i32) -> i32; },
                , :: newer_type, (i32, fn(i32) -> i32), Repeater, 13541114746309738512u64
            }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__7809343176067811145"
            && e.input == r#"(for < 'a, A, B > AdvancedFreeParam < 'a, A, B, String > where A : Clone +
            Debug, B : PartialEq < i32 >) struct AdvancedFreeParamNewType(BasicType);"#
            && e.to == r#":: newer_type :: __implement_internal!
            {
                ((for < 'a, A, B > AdvancedFreeParam < 'a, A, B, String > where A : Clone
                + Debug, B : PartialEq < i32 >) struct
                AdvancedFreeParamNewType(BasicType);) trait AdvancedFreeParam < 'a, A, B,
                C > where A : :: core :: clone :: Clone + :: core :: fmt :: Debug, B : ::
                core :: cmp :: PartialEq < i32 > , C : :: core :: default :: Default,
                { fn advanced_method(& self, input : & 'a A, flag : B) -> C; }, , ::
                newer_type, (i32), Repeater, 7809343176067811145u64
            }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__15653759549052608261"
            && e.input == r#"(for < 'a, A > FreeParamTrait < 'a, A, u32 > where A : Clone) struct
            FreeParamNewType(BasicType);"#
            && e.to == r#":: newer_type :: __implement_internal!
            {
                ((for < 'a, A > FreeParamTrait < 'a, A, u32 > where A : Clone) struct
                FreeParamNewType(BasicType);) trait FreeParamTrait < 'a, A, B > where A :
                :: core :: clone :: Clone,
                { fn complex_method(& self, input : & 'a A) -> B; }, , :: newer_type, (),
                Repeater, 15653759549052608261u64
            }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__15370362212820693701"
            && e.input == r#"(UltimateTrait < String, i32 >) struct UltimateNewType(BasicType);"#
            && e.to == r#":: newer_type :: __implement_internal!
            {
                ((UltimateTrait < String, i32 >) struct UltimateNewType(BasicType);) trait
                UltimateTrait < T, U > where T : :: core :: fmt :: Debug + :: core ::
                clone :: Clone, U : :: core :: cmp :: PartialEq,
                { fn combine(& self, a : T, b : U) -> (T, bool); }, , :: newer_type,
                (bool), Repeater, 15370362212820693701u64
            }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__324955138424952050"
            && e.input == r#"(ComplexTrait < String >) struct ComplexNewType(BasicType);"#
            && e.to == r#":: newer_type :: __implement_internal!
           {
               ((ComplexTrait < String >) struct ComplexNewType(BasicType);) trait
               ComplexTrait < T > where T : :: core :: clone :: Clone + :: core :: fmt ::
               Debug, { fn describe(& self, item : T) -> :: std :: string :: String; }, ,
               :: newer_type, (), Repeater, 324955138424952050u64
           }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__186050351402150740"
            && e.input == r#"(AdvancedTrait < i32 >) struct AdvancedNewType(BasicType);"#
            && e.to == r#":: newer_type :: __implement_internal!
           {
               ((AdvancedTrait < i32 >) struct AdvancedNewType(BasicType);) trait
               AdvancedTrait < T >
               { fn compute < U > (& self, value : T, extra : U) -> (T, U); }, , ::
               newer_type, (), Repeater, 186050351402150740u64
           }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__3696430908058849413"
            && e.input == r#"(GenericTrait < i32 >) struct GenericNewType(BasicType);"#
            && e.to == r#":: newer_type :: __implement_internal!
           {
               ((GenericTrait < i32 >) struct GenericNewType(BasicType);) trait
               GenericTrait < T > { fn process(& self, input : T) -> T; }, , ::
               newer_type, (), Repeater, 3696430908058849413u64
           }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__16719574466313965748"
            && e.input == r#"(BasicTrait) struct BasicNewType(BasicType);"#
            && e.to == r#":: newer_type :: __implement_internal!
           {
               ((BasicTrait) struct BasicNewType(BasicType);) trait BasicTrait
               {
                   fn get_number(& self) -> i32; fn double_number(& self) -> i32
                   { self.get_number() * 2 }
               }, , :: newer_type, (i32), Repeater, 16719574466313965748u64
           }"#
    }));
}

#[test]
fn external_crate_newer_type_test_test4() {
    let expansions = run_trace_for_repo("newer-type", Some("test4"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "implement"
            && e.input == r#"struct ComplexNewType(AdvancedType);"#
            && e.to == r#"struct ComplexNewType(AdvancedType); ComplexTrait!
{ (ComplexTrait) struct ComplexNewType(AdvancedType); }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "target"
            && e.input == r#"trait ComplexTrait
{
    const SCALE: i32; type Output; fn compute(&self, input: i32) ->
    Self::Output;
}"#
            && e.to == r#"#[doc(hidden)] #[macro_export] macro_rules!
__newer_type_macro__6455442166797493642
{
    ($ ($t : tt) *) =>
    {
        :: newer_type :: __implement_internal!
        {
            ($ ($t) *) trait ComplexTrait
            {
                const SCALE : i32; type Output; fn
                compute(& self, input : i32) -> Self :: Output;
            }, , :: newer_type, (i32), Repeater, 6455442166797493642u64
        }
    }
} #[doc(hidden)] use __newer_type_macro__6455442166797493642 as ComplexTrait;
#[allow(private_bounds)] #[allow(clippy :: missing_safety_doc)] trait
ComplexTrait
{
    const SCALE : i32; type Output; fn compute(& self, input : i32) -> Self ::
    Output;
} impl < __NewerTypeSelf6455442166797493642 > Repeater
<6455442166797493642u64, 0usize, () > for __NewerTypeSelf6455442166797493642
where Self : ComplexTrait, { type Type = i32; }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__implement_internal"
            && e.input == r#"((for < 'a, A > FreeParamComplex < 'a, A, String > where A : Debug + Clone)
struct FreeParamComplexNewType(AdvancedType);) trait FreeParamComplex < 'a, A,
B > where A : :: core :: fmt :: Debug + :: core :: clone :: Clone, B : :: core
:: default :: Default,
{
    const MULTIPLIER : i32; type Output; fn perform(& self, input : & 'a A) ->
    (Self :: Output, B);
}, , :: newer_type, (i32), Repeater, 2067555396234330445u64"#
            && e.to == r#"#[automatically_derived] impl < 'a_newer_type_6900820963442800081,
NewerTypeTypeParamAOf6900820963442800081 > FreeParamComplex <
'a_newer_type_6900820963442800081, NewerTypeTypeParamAOf6900820963442800081,
String > for FreeParamComplexNewType where
NewerTypeTypeParamAOf6900820963442800081 : Debug + Clone, AdvancedType :
FreeParamComplex <'a_newer_type_6900820963442800081,
NewerTypeTypeParamAOf6900820963442800081, String, >
{
    const MULTIPLIER : i32 = <AdvancedType as FreeParamComplex <
    'a_newer_type_6900820963442800081,
    NewerTypeTypeParamAOf6900820963442800081, String > >::MULTIPLIER; type
    Output = <AdvancedType as FreeParamComplex <
    'a_newer_type_6900820963442800081,
    NewerTypeTypeParamAOf6900820963442800081, String > >::Output; fn
    perform(& self, input : & 'a_newer_type_6900820963442800081
    NewerTypeTypeParamAOf6900820963442800081) -> (Self :: Output, String)
    {
        let Self(__newer_type_pred_param_0) = self; < _ as FreeParamComplex <
        'a_newer_type_6900820963442800081,
        NewerTypeTypeParamAOf6900820963442800081, String > > ::
        perform(__newer_type_pred_param_0, input)
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__2067555396234330445"
            && e.input == r#"(for < 'a, A > FreeParamComplex < 'a, A, String > where A : Debug + Clone)
            struct FreeParamComplexNewType(AdvancedType);"#
            && e.to == r#":: newer_type :: __implement_internal!
            {
                ((for < 'a, A > FreeParamComplex < 'a, A, String > where A : Debug +
                Clone) struct FreeParamComplexNewType(AdvancedType);) trait
                FreeParamComplex < 'a, A, B > where A : :: core :: fmt :: Debug + :: core
                :: clone :: Clone, B : :: core :: default :: Default,
                {
                    const MULTIPLIER : i32; type Output; fn
                    perform(& self, input : & 'a A) -> (Self :: Output, B);
                }, , :: newer_type, (i32), Repeater, 2067555396234330445u64
            }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__14835590545481260405"
            && e.input == r#"(ConstrainedTrait < String >) struct ConstrainedNewType(AdvancedType);"#
            && e.to == r#":: newer_type :: __implement_internal!
           {
               ((ConstrainedTrait < String >) struct ConstrainedNewType(AdvancedType);)
               trait ConstrainedTrait < T > where T : :: core :: fmt :: Debug + :: core
               :: clone :: Clone + :: core :: default :: Default,
               {
                   const LIMIT : usize; type Item; fn process(& self, input : T) -> Self
                   :: Item;
               }, , :: newer_type, (usize), Repeater, 14835590545481260405u64
           }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__5787597172021426869"
            && e.input == r#"(MultiAssocTrait < i32 >) struct MultiAssocNewType(AdvancedType);"#
            && e.to == r#":: newer_type :: __implement_internal!
           {
               ((MultiAssocTrait < i32 >) struct MultiAssocNewType(AdvancedType);) trait
               MultiAssocTrait < T >
               {
                   const FACTOR : T; type Result; fn transform(& self, input : T) -> Self
                   :: Result;
               }, , :: newer_type, (), Repeater, 5787597172021426869u64
           }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__6455442166797493642"
            && e.input == r#"(ComplexTrait) struct ComplexNewType(AdvancedType);"#
            && e.to == r#":: newer_type :: __implement_internal!
           {
               ((ComplexTrait) struct ComplexNewType(AdvancedType);) trait ComplexTrait
               {
                   const SCALE : i32; type Output; fn compute(& self, input : i32) ->
                   Self :: Output;
               }, , :: newer_type, (i32), Repeater, 6455442166797493642u64
           }"#
    }));
}

#[test]
fn external_crate_newer_type_test_test5() {
    let expansions = run_trace_for_repo("newer-type", Some("test5"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "implement"
            && e.input == r#"#[allow(unused)] struct MyWrapper(String);"#
            && e.to == r#"#[allow(unused)] struct MyWrapper(String); m :: MyNewTrait!
{ (m :: MyNewTrait) #[allow(unused)] struct MyWrapper(String); }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "target"
            && e.input == r#"pub trait MyNewTrait
{
    type MyType<'a> where Self: 'a; fn get<'a>(&'a self, a: T) ->
    Self::MyType<'a>;
}"#
            && e.to == r#"#[doc(hidden)] #[macro_export] macro_rules!
__newer_type_macro__4108544471553603643
{
    ($ ($t : tt) *) =>
    {
        :: newer_type :: __implement_internal!
        {
            ($ ($t) *) pub trait MyNewTrait
            {
                type MyType < 'a > where Self : 'a; fn get < 'a >
                (& 'a self, a : T) -> Self :: MyType < 'a > ;
            }, , :: newer_type, (T), crate :: Repeater, 4108544471553603643u64
        }
    }
} #[doc(hidden)] pub use __newer_type_macro__4108544471553603643 as
MyNewTrait; #[allow(private_bounds)] #[allow(clippy :: missing_safety_doc)]
pub trait MyNewTrait
{
    type MyType < 'a > where Self : 'a; fn get < 'a > (& 'a self, a : T) ->
    Self :: MyType < 'a > ;
} impl < __NewerTypeSelf4108544471553603643 > crate :: Repeater
<4108544471553603643u64, 0usize, () > for __NewerTypeSelf4108544471553603643
where Self : MyNewTrait, { type Type = T; }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__implement_internal"
            && e.input == r#"((m :: MyNewTrait) #[allow(unused)] struct MyWrapper(String);) pub trait
MyNewTrait
{
    type MyType < 'a > where Self : 'a; fn get < 'a > (& 'a self, a : T) ->
    Self :: MyType < 'a > ;
}, , :: newer_type, (T), crate :: Repeater, 4108544471553603643u64"#
            && e.to == r#"#[automatically_derived] impl < > m :: MyNewTrait for MyWrapper where String :
m :: MyNewTrait <>
{
    type MyType < 'a > = <String as m :: MyNewTrait >::MyType < 'a > where
    Self : 'a; fn get < 'a >
    (& 'a self, a : < Self as crate :: Repeater < 4108544471553603643u64,
    0usize, () > > :: Type) -> Self :: MyType < 'a >
    {
        let Self(__newer_type_pred_param_0) = self; < _ as m :: MyNewTrait >
        :: get(__newer_type_pred_param_0, a)
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__newer_type_macro__4108544471553603643"
            && e.input == r#"(m :: MyNewTrait) #[allow(unused)] struct MyWrapper(String);"#
            && e.to == r#":: newer_type :: __implement_internal!
           {
               ((m :: MyNewTrait) #[allow(unused)] struct MyWrapper(String);) pub trait
               MyNewTrait
               {
                   type MyType < 'a > where Self : 'a; fn get < 'a > (& 'a self, a : T)
                   -> Self :: MyType < 'a > ;
               }, , :: newer_type, (T), crate :: Repeater, 4108544471553603643u64
           }"#
    }));
}

#[test]
fn external_crate_parametrized_show_expansion() {
    let expansions = run_trace_for_repo("parametrized", None);
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && e.input == r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#
            && e.to == r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_11177185773735460263
{
    ($ ($t : tt) *) =>
    {
        $crate :: _sumtrait_internal!
        ({ $ ($t) * } [],
        {
            #[doc =
            " Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
            #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8])
                -> :: std :: io :: Result < :: core :: primitive :: usize > ;
            }
        }, 10791577178322682870usize, $crate, $crate :: traits :: Marker,
        [:: std :: io :: Read], [], [],);
    };
} #[doc(hidden)] pub use __sumtype_macro_11177185773735460263 as Read;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_impl_trait"
            && e.input == r#"[map, into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self = Vec<T>,
            self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }
            {
                f = f, T = M, Mapped = Vec<M>, param_map =
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() },
            }"#
            && e.to == r#"impl < T, M > ParametrizedMap < 0, M > for Vec<T>
            {
                type Mapped = Vec<M>; fn
                param_map(self, f : impl FnMut(Self :: Item) -> M) -> Self :: Mapped
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() }
            } emit_impl_trait!
            ([into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self = Vec<T>, self =
            self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            });"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "impl_all"
            && e.input == r#"[T] map, into_iter, iter_mut for Vec<T>, T = M, Mapped = Vec<M>; [T] into_iter
            for std::collections::BTreeSet<T>; [T] into_iter for
            std::collections::HashSet<T>; [T] into_iter for
            std::collections::BinaryHeap<T>; [T] map, into_iter, iter_mut for
            std::collections::LinkedList<T>, T = M, Mapped =
            std::collections::LinkedList<M>; [T] map, into_iter, iter_mut for
            std::collections::VecDeque<T>, T = M, Mapped = std::collections::VecDeque<M>;"#
            && e.to == r#"emit_impl_trait!
            ([map, into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self = Vec<T>,
            self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }
            {
                f = f, T = M, Mapped = Vec<M>, param_map =
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() },
            }); emit_impl_trait!
            ([into_iter,] impl_generics = [T], PARAM = 0, Self =
            std::collections::BTreeSet<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }); emit_impl_trait!
            ([into_iter,] impl_generics = [T], PARAM = 0, Self =
            std::collections::HashSet<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }); emit_impl_trait!
            ([into_iter,] impl_generics = [T], PARAM = 0, Self =
            std::collections::BinaryHeap<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }); emit_impl_trait!
            ([map, into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self =
            std::collections::LinkedList<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }
            {
                f = f, T = M, Mapped = std::collections::LinkedList<M>, param_map =
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() },
            }); emit_impl_trait!
            ([map, into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self =
            std::collections::VecDeque<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }
            {
                f = f, T = M, Mapped = std::collections::VecDeque<M>, param_map =
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() },
            });"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "impl_for_tuple"
            && e.input == r#"[] T []"#
            && e.to == r#"impl < T > Parametrized < {impl_for_tuple! (@ count)}> for (T,)
            {
                type Item = T; const MIN_LEN : usize = 1; const MAX_LEN : Option < usize >
                = Some(1); fn param_len(& self) -> usize { 1 } type Iter < 'a > = :: core
                :: iter :: Once < & 'a Self :: Item > where (Self, Self :: Item): 'a; fn
                param_iter < 'a > (& 'a self) -> Self :: Iter < 'a > where Self :: Item :
                'a
                {
                    :: core :: iter ::
                    once(impl_for_tuple!
                    (@ nth []
                    [& self.0, & self.1, & self.2, & self.3, & self.4, & self.5, & self.6,
                    & self.7, & self.8, & self.9, & self.10, & self.11]))
                }
            } impl < T > ParametrizedIterMut < {impl_for_tuple! (@ count)}> for (T,)
            {
                type IterMut < 'a > = :: core :: iter :: Once < & 'a mut Self :: Item >
                where (Self, Self :: Item): 'a; fn param_iter_mut < 'a > (& 'a mut self)
                -> Self :: IterMut < 'a > where Self :: Item : 'a
                {
                    core :: iter ::
                    once(impl_for_tuple!
                    (@ nth []
                    [& mut self.0, & mut self.1, & mut self.2, & mut self.3, & mut self.4,
                    & mut self.5, & mut self.6, & mut self.7, & mut self.8, & mut self.9,
                    & mut self.10, & mut self.11]))
                }
            } impl < T > ParametrizedIntoIter < {impl_for_tuple! (@ count)}> for (T,)
            {
                type IntoIter = :: core :: iter :: Once < Self :: Item > where Self ::
                Item : Sized; fn param_into_iter(self) -> Self :: IntoIter where Self ::
                Item : Sized
                {
                    core :: iter ::
                    once(impl_for_tuple!
                    (@ nth []
                    [self.0, self.1, self.2, self.3, self.4, self.5, self.6, self.7,
                    self.8, self.9, self.10, self.11]))
                }
            } impl < U, T > ParametrizedMap < {impl_for_tuple! (@ count)}, U > for (T,)
            {
                type Mapped = (U,); fn
                param_map(self, mut f : impl FnMut(Self :: Item) -> U) -> Self :: Mapped
                where Self :: Item : Sized
                {
                    impl_for_tuple!
                    (@ wrap_f f [] T []
                    [self.0, self.1, self.2, self.3, self.4, self.5, self.6, self.7,
                    self.8, self.9, self.10, self.11] {})
                }
            }"#
    }));
}

#[test]
fn external_crate_parametrized_test_flatten_bug() {
    let expansions = run_trace_for_repo("parametrized", Some("flatten_bug"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "parametrized"
            && e.input == r#"#[allow(unused)] enum MyEnum<A> { E1(A), E2((A,)), }"#
            && e.to == r#"#[allow(unused)] enum MyEnum < A > { E1(A), E2((A,)), }
#[:: parametrized :: _imp :: sumtype ::
sumtype(:: parametrized :: _imp :: sumtype :: traits :: Iterator)] impl < A >
:: parametrized :: Parametrized <0usize > for MyEnum < A >
{
    type Item = A; const MIN_LEN : usize =
    {
        const fn __parametric_type_min(a : usize, b : usize) -> usize
        { if a < b { a } else { b } }
        __parametric_type_min(1usize, < (A,) as :: parametrized ::
        Parametrized < 0usize > > :: MIN_LEN * 1usize)
    }; const MAX_LEN : Option < usize > =
    {
        const fn
        __parametric_type_max(a : Option < usize > , b : Option < usize >) ->
        Option < usize >
        {
            match (a, b)
            {
                (Some(a), Some(b)) => if a > b { Some(a) } else { Some(b) } _
                => None,
            }
        }
        __parametric_type_max(:: core :: option :: Option :: Some(1usize), if
        let (Some(l), Some(r)) =
        (< (A,) as :: parametrized :: Parametrized < 0usize > > :: MAX_LEN, ::
        core :: option :: Option :: Some(1usize)) { Some(l * r) } else
        { None })
    }; fn param_len(& self) -> usize
    {
        #[allow(unused)] match self
        {
            MyEnum ::E1(__parametric_type_id_0) => { 1usize } MyEnum
            ::E2(__parametric_type_id_0) =>
            {
                < (A,) as :: parametrized :: Parametrized < 0usize > > ::
                param_iter(__parametric_type_id_0).map(| __parametrized_arg |
                1usize).sum :: < :: core :: primitive :: usize > ()
            }
        }
    } type Iter <'__parametrized_lt > = sumtype! ['__parametrized_lt] where
    (Self, Self :: Item) : '__parametrized_lt; fn param_iter <
    '__parametrized_lt > (& '__parametrized_lt self) -> Self :: Iter <
    '__parametrized_lt > where Self :: Item : '__parametrized_lt
    {
        #[allow(unused)] match self
        {
            MyEnum ::E1(__parametric_type_id_0) =>
            {
                sumtype!
                (:: core :: iter :: once(__parametric_type_id_0), for
                <'__parametrized_lt > :: core :: iter :: Once < &
                '__parametrized_lt A > where A : '__parametrized_lt,)
            } MyEnum ::E2(__parametric_type_id_0) =>
            {
                sumtype!
                ({
                    let __parametrized_fn : fn(& '__parametrized_lt A) -> _ = |
                    __parametrized_arg |
                    { :: core :: iter :: once(__parametrized_arg) }; ::
                    parametrized :: Flatten ::
                    new(< (A,) as :: parametrized :: Parametrized < 0usize > >
                    ::
                    param_iter(__parametric_type_id_0).map(__parametrized_fn))
                }, for <'__parametrized_lt > :: parametrized :: Flatten < ::
                core :: iter :: Map < < (A,) as :: parametrized ::
                Parametrized < 0usize > > :: Iter < '__parametrized_lt > ,
                fn(& '__parametrized_lt A) -> :: core :: iter :: Once < &
                '__parametrized_lt A > > , :: core :: iter :: Once < &
                '__parametrized_lt A > > where A : '__parametrized_lt,)
            }
        }
    }
}
#[:: parametrized :: _imp :: sumtype ::
sumtype(:: parametrized :: _imp :: sumtype :: traits :: Iterator)] impl < A >
:: parametrized :: ParametrizedIterMut <0usize > for MyEnum < A >
{
    type IterMut <'__parametrized_lt > = sumtype! ['__parametrized_lt] where
    (Self, Self :: Item) : '__parametrized_lt; fn param_iter_mut <
    '__parametrized_lt > (& '__parametrized_lt mut self) -> Self :: IterMut <
    '__parametrized_lt > where Self :: Item : '__parametrized_lt
    {
        #[allow(unused)] match self
        {
            MyEnum ::E1(__parametric_type_id_0) =>
            {
                sumtype!
                (:: core :: iter :: once(__parametric_type_id_0), for
                <'__parametrized_lt > :: core :: iter :: Once < &
                '__parametrized_lt mut A > where A : '__parametrized_lt,)
            } MyEnum ::E2(__parametric_type_id_0) =>
            {
                sumtype!
                (:: core :: iter :: once(& mut __parametric_type_id_0.0), for
                <'__parametrized_lt > :: core :: iter :: Once < &
                '__parametrized_lt mut A > where A : '__parametrized_lt,)
            }
        }
    }
}
#[:: parametrized :: _imp :: sumtype ::
sumtype(:: parametrized :: _imp :: sumtype :: traits :: Iterator)] impl < A >
:: parametrized :: ParametrizedIntoIter <0usize > for MyEnum < A >
{
    type IntoIter = sumtype! []; fn param_into_iter(self) -> Self :: IntoIter
    {
        #[allow(unused)] match self
        {
            MyEnum ::E1(__parametric_type_id_0) =>
            {
                sumtype!
                (:: core :: iter :: once(__parametric_type_id_0), :: core ::
                iter :: Once < A >)
            } MyEnum ::E2(__parametric_type_id_0) =>
            {
                sumtype!
                (:: core :: iter :: once(__parametric_type_id_0.0), :: core ::
                iter :: Once < A >)
            }
        }
    }
} impl < A, __PARAMETRIZED_MAP_PARAM > :: parametrized :: ParametrizedMap
<0usize, __PARAMETRIZED_MAP_PARAM > for MyEnum < A >
{
    type Mapped = MyEnum < __PARAMETRIZED_MAP_PARAM > ; fn
    param_map(self, mut __parametrized_map_fn : impl FnMut(Self :: Item) ->
    __PARAMETRIZED_MAP_PARAM) -> Self :: Mapped where Self :: Item : :: core
    :: marker :: Sized
    {
        match self
        {
            MyEnum ::E1(__parametric_type_id_0) =>
            { MyEnum ::E1(__parametrized_map_fn(__parametric_type_id_0)) }
            MyEnum ::E2(__parametric_type_id_0) =>
            {
                MyEnum
                ::E2(< (A,) as :: parametrized :: ParametrizedMap < 0usize,
                __PARAMETRIZED_MAP_PARAM > > ::
                param_map(__parametric_type_id_0, | __parametrized_arg |
                { __parametrized_map_fn(__parametrized_arg) }))
            }
        }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && e.input == r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#
            && e.to == r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_11177185773735460263
{
    ($ ($t : tt) *) =>
    {
        $crate :: _sumtrait_internal!
        ({ $ ($t) * } [],
        {
            #[doc =
            " Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
            #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8])
                -> :: std :: io :: Result < :: core :: primitive :: usize > ;
            }
        }, 10791577178322682870usize, $crate, $crate :: traits :: Marker,
        [:: std :: io :: Read], [], [],);
    };
} #[doc(hidden)] pub use __sumtype_macro_11177185773735460263 as Read;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtype"
            && e.input == r#"impl < A > :: parametrized :: Parametrized <0usize > for MyEnum < A >
{
    type Item = A; const MIN_LEN : usize =
    {
        const fn __parametric_type_min(a : usize, b : usize) -> usize
        { if a < b { a } else { b } }
        __parametric_type_min(1usize, < (A,) as :: parametrized ::
        Parametrized < 0usize > > :: MIN_LEN * 1usize)
    }; const MAX_LEN : Option < usize > =
    {
        const fn
        __parametric_type_max(a : Option < usize > , b : Option < usize >) ->
        Option < usize >
        {
            match (a, b)
            {
                (Some(a), Some(b)) => if a > b { Some(a) } else { Some(b) } _
                => None,
            }
        }
        __parametric_type_max(:: core :: option :: Option :: Some(1usize), if
        let (Some(l), Some(r)) =
        (< (A,) as :: parametrized :: Parametrized < 0usize > > :: MAX_LEN, ::
        core :: option :: Option :: Some(1usize)) { Some(l * r) } else
        { None })
    }; fn param_len(& self) -> usize
    {
        #[allow(unused)] match self
        {
            MyEnum ::E1(__parametric_type_id_0) => { 1usize } MyEnum
            ::E2(__parametric_type_id_0) =>
            {
                < (A,) as :: parametrized :: Parametrized < 0usize > > ::
                param_iter(__parametric_type_id_0).map(| __parametrized_arg |
                1usize).sum :: < :: core :: primitive :: usize > ()
            }
        }
    } type Iter <'__parametrized_lt > = sumtype! ['__parametrized_lt] where
    (Self, Self :: Item) : '__parametrized_lt; fn param_iter <
    '__parametrized_lt > (& '__parametrized_lt self) -> Self :: Iter <
    '__parametrized_lt > where Self :: Item : '__parametrized_lt
    {
        #[allow(unused)] match self
        {
            MyEnum ::E1(__parametric_type_id_0) =>
            {
                sumtype!
                (:: core :: iter :: once(__parametric_type_id_0), for
                <'__parametrized_lt > :: core :: iter :: Once < &
                '__parametrized_lt A > where A : '__parametrized_lt,)
            } MyEnum ::E2(__parametric_type_id_0) =>
            {
                sumtype!
                ({
                    let __parametrized_fn : fn(& '__parametrized_lt A) -> _ = |
                    __parametrized_arg |
                    { :: core :: iter :: once(__parametrized_arg) }; ::
                    parametrized :: Flatten ::
                    new(< (A,) as :: parametrized :: Parametrized < 0usize > >
                    ::
                    param_iter(__parametric_type_id_0).map(__parametrized_fn))
                }, for <'__parametrized_lt > :: parametrized :: Flatten < ::
                core :: iter :: Map < < (A,) as :: parametrized ::
                Parametrized < 0usize > > :: Iter < '__parametrized_lt > ,
                fn(& '__parametrized_lt A) -> :: core :: iter :: Once < &
                '__parametrized_lt A > > , :: core :: iter :: Once < &
                '__parametrized_lt A > > where A : '__parametrized_lt,)
            }
        }
    }
}"#
            && e.to == r#"#[doc(hidden)] #[allow(non_camel_case_types)] #[allow(non_camel_case_types)]
struct __SumType_RefType_14603428086114329038_0; #[doc(hidden)]
#[allow(non_camel_case_types)] #[allow(non_camel_case_types)] struct
__SumType_RefType_1820574786928708003_1; #[doc(hidden)]
#[allow(non_camel_case_types)] trait
__Sumtype_TypeRef_Trait_3652118842108341957 <'__parametrized_lt, A :
'__parametrized_lt > { type Type; } #[doc(hidden)]
#[allow(non_camel_case_types)] pub enum __Sumtype_Enum_3652118842108341957 <
'__parametrized_lt, A : '__parametrized_lt >
{
    __SumType_Variant_0(< __SumType_RefType_14603428086114329038_0 as
    __Sumtype_TypeRef_Trait_3652118842108341957 < '__parametrized_lt, A > > ::
    Type),
    __SumType_Variant_1(< __SumType_RefType_1820574786928708003_1 as
    __Sumtype_TypeRef_Trait_3652118842108341957 < '__parametrized_lt, A > > ::
    Type),
    __Uninhabited((:: core :: convert :: Infallible, :: core :: marker ::
    PhantomData <& '__parametrized_lt () > , :: core :: marker :: PhantomData
    <A >)),
} #[doc(hidden)] #[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_3652118842108341957 <'__parametrized_lt, A :
'__parametrized_lt > {} impl <'__parametrized_lt, A : '__parametrized_lt,
__Sumtype_TypeParam > __Sumtype_ConstraintExprTrait_3652118842108341957
<'__parametrized_lt, A > for __Sumtype_TypeParam where __Sumtype_TypeParam :
__Sumtype_ConstraintExprTrait_0_5457517761398843393 <'__parametrized_lt, A >,
A : '__parametrized_lt, {} :: parametrized :: _imp :: sumtype :: traits ::
Iterator!
(__Sumtype_ConstraintExprTrait_0_5457517761398843393, :: parametrized :: _imp
:: sumtype :: traits :: Iterator, __Sumtype_Enum_3652118842108341957, [],
[__SumType_Variant_0 :< __SumType_RefType_14603428086114329038_0 as
__Sumtype_TypeRef_Trait_3652118842108341957 < '__parametrized_lt, A > > ::
Type, __SumType_Variant_1 :< __SumType_RefType_1820574786928708003_1 as
__Sumtype_TypeRef_Trait_3652118842108341957 < '__parametrized_lt, A > > ::
Type], ['__parametrized_lt, A : '__parametrized_lt], ['__parametrized_lt, A],
{ A : '__parametrized_lt },); #[allow(non_local_definitions)] impl < A > ::
parametrized :: Parametrized < 0usize > for MyEnum < A >
{
    type Item = A; const MIN_LEN : usize =
    {
        const fn __parametric_type_min(a : usize, b : usize) -> usize
        { if a < b { a } else { b } }
        __parametric_type_min(1usize, < (A,) as :: parametrized ::
        Parametrized < 0usize > > :: MIN_LEN * 1usize)
    }; const MAX_LEN : Option < usize > =
    {
        const fn
        __parametric_type_max(a : Option < usize > , b : Option < usize >) ->
        Option < usize >
        {
            match (a, b)
            {
                (Some(a), Some(b)) => if a > b { Some(a) } else { Some(b) } _
                => None,
            }
        }
        __parametric_type_max(:: core :: option :: Option :: Some(1usize), if
        let (Some(l), Some(r)) =
        (< (A,) as :: parametrized :: Parametrized < 0usize > > :: MAX_LEN, ::
        core :: option :: Option :: Some(1usize)) { Some(l * r) } else
        { None })
    }; fn param_len(& self) -> usize
    {
        #[allow(unused)] match self
        {
            MyEnum :: E1(__parametric_type_id_0) => { 1usize } MyEnum ::
            E2(__parametric_type_id_0) =>
            {
                < (A,) as :: parametrized :: Parametrized < 0usize > > ::
                param_iter(__parametric_type_id_0).map(| __parametrized_arg |
                1usize).sum :: < :: core :: primitive :: usize > ()
            }
        }
    } type Iter < '__parametrized_lt > = __Sumtype_Enum_3652118842108341957 <
    '__parametrized_lt, A > where (Self, Self :: Item) : '__parametrized_lt;
    fn param_iter < '__parametrized_lt > (& '__parametrized_lt self) -> Self
    :: Iter < '__parametrized_lt > where Self :: Item : '__parametrized_lt
    {
        #[allow(unused)] match self
        {
            MyEnum :: E1(__parametric_type_id_0) =>
            {
                {
                    impl < '__parametrized_lt, A, >
                    __Sumtype_TypeRef_Trait_3652118842108341957 <
                    '__parametrized_lt, A > for
                    __SumType_RefType_14603428086114329038_0 where A :
                    '__parametrized_lt,
                    {
                        type Type = :: core :: iter :: Once < & '__parametrized_lt A
                        > ;
                    } fn __sum_type_id_fn_12394162291220834937 <
                    '__parametrized_lt, A, __SumType_T :
                    __Sumtype_ConstraintExprTrait_3652118842108341957 <
                    '__parametrized_lt, A > > (t : __SumType_T) -> __SumType_T
                    where A : '__parametrized_lt, { t }
                    __sum_type_id_fn_12394162291220834937 :: <
                    '__parametrized_lt, A, _ >
                    (__Sumtype_Enum_3652118842108341957 ::
                    __SumType_Variant_0(:: core :: iter ::
                    once(__parametric_type_id_0)))
                }
            } MyEnum :: E2(__parametric_type_id_0) =>
            {
                {
                    impl < '__parametrized_lt, A, >
                    __Sumtype_TypeRef_Trait_3652118842108341957 <
                    '__parametrized_lt, A > for
                    __SumType_RefType_1820574786928708003_1 where A :
                    '__parametrized_lt,
                    {
                        type Type = :: parametrized :: Flatten < :: core :: iter ::
                        Map < < (A,) as :: parametrized :: Parametrized < 0usize > >
                        :: Iter < '__parametrized_lt > , fn(& '__parametrized_lt A)
                        -> :: core :: iter :: Once < & '__parametrized_lt A > > , ::
                        core :: iter :: Once < & '__parametrized_lt A > > ;
                    } fn __sum_type_id_fn_10214525939857987730 <
                    '__parametrized_lt, A, __SumType_T :
                    __Sumtype_ConstraintExprTrait_3652118842108341957 <
                    '__parametrized_lt, A > > (t : __SumType_T) -> __SumType_T
                    where A : '__parametrized_lt, { t }
                    __sum_type_id_fn_10214525939857987730 :: <
                    '__parametrized_lt, A, _ >
                    (__Sumtype_Enum_3652118842108341957 ::
                    __SumType_Variant_1({
                        let __parametrized_fn : fn(& '__parametrized_lt A) -> _ = |
                        __parametrized_arg |
                        { :: core :: iter :: once(__parametrized_arg) }; ::
                        parametrized :: Flatten ::
                        new(< (A,) as :: parametrized :: Parametrized < 0usize > >
                        ::
                        param_iter(__parametric_type_id_0).map(__parametrized_fn))
                    }))
                }
            }
        }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__sumtype_macro_5221274140601386781"
            && e.input == r#"__Sumtype_ConstraintExprTrait_0_5457517761398843393, :: parametrized :: _imp
          :: sumtype :: traits :: Iterator, __Sumtype_Enum_3652118842108341957, [],
          [__SumType_Variant_0 :< __SumType_RefType_14603428086114329038_0 as
          __Sumtype_TypeRef_Trait_3652118842108341957 < '__parametrized_lt, A > > ::
          Type, __SumType_Variant_1 :< __SumType_RefType_1820574786928708003_1 as
          __Sumtype_TypeRef_Trait_3652118842108341957 < '__parametrized_lt, A > > ::
          Type], ['__parametrized_lt, A : '__parametrized_lt], ['__parametrized_lt, A],
          { A : '__parametrized_lt },"#
            && e.to == r#"$crate :: _sumtrait_internal!
          ({
              __Sumtype_ConstraintExprTrait_0_5457517761398843393, :: parametrized ::
              _imp :: sumtype :: traits :: Iterator, __Sumtype_Enum_3652118842108341957,
              [],
              [__SumType_Variant_0 :< __SumType_RefType_14603428086114329038_0 as
              __Sumtype_TypeRef_Trait_3652118842108341957 < '__parametrized_lt, A > > ::
              Type, __SumType_Variant_1 :< __SumType_RefType_1820574786928708003_1 as
              __Sumtype_TypeRef_Trait_3652118842108341957 < '__parametrized_lt, A > > ::
              Type], ['__parametrized_lt, A : '__parametrized_lt],
              ['__parametrized_lt, A], { A : '__parametrized_lt },
          } [],
          {
              #[doc =
              " Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`]."]
              #[allow(private_bounds)] pub trait Iterator
              {
                  type Item; fn next(& mut self) -> :: core :: option :: Option < Self
                  :: Item > ;
              }
          }, 15686212630352170898usize, $crate, $crate :: traits :: Marker,
          [:: core :: iter :: Iterator], [], [],);"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "_sumtrait_internal"
            && e.input == r#"{
    __Sumtype_ConstraintExprTrait_0_5457517761398843393, :: parametrized ::
    _imp :: sumtype :: traits :: Iterator, __Sumtype_Enum_3652118842108341957,
    [],
    [__SumType_Variant_0 :< __SumType_RefType_14603428086114329038_0 as
    __Sumtype_TypeRef_Trait_3652118842108341957 < '__parametrized_lt, A > > ::
    Type, __SumType_Variant_1 :< __SumType_RefType_1820574786928708003_1 as
    __Sumtype_TypeRef_Trait_3652118842108341957 < '__parametrized_lt, A > > ::
    Type], ['__parametrized_lt, A : '__parametrized_lt],
    ['__parametrized_lt, A], { A : '__parametrized_lt },
} [],
{
    #[doc =
    " Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`]."]
    #[allow(private_bounds)] pub trait Iterator
    {
        type Item; fn next(& mut self) -> :: core :: option :: Option < Self
        :: Item > ;
    }
}, 15686212630352170898usize, $crate, $crate :: traits :: Marker,
[:: core :: iter :: Iterator], [], [],"#
            && e.to == r#"#[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_0_5457517761398843393 <'__parametrized_lt, A :
'__parametrized_lt > {} impl <'__parametrized_lt, A : '__parametrized_lt,
__SumType_AssocType_Item > __Sumtype_ConstraintExprTrait_0_5457517761398843393
<'__parametrized_lt, A > for __Sumtype_Enum_3652118842108341957
<'__parametrized_lt, A > where A : '__parametrized_lt, <
__SumType_RefType_14603428086114329038_0 as
__Sumtype_TypeRef_Trait_3652118842108341957 < '__parametrized_lt, A > > ::
Type : :: core :: iter :: Iterator < Item = __SumType_AssocType_Item >, <
__SumType_RefType_1820574786928708003_1 as
__Sumtype_TypeRef_Trait_3652118842108341957 < '__parametrized_lt, A > > ::
Type : :: core :: iter :: Iterator < Item = __SumType_AssocType_Item >, {}
impl <'__parametrized_lt, A : '__parametrized_lt, __SumType_AssocType_Item >
:: core :: iter :: Iterator <> for __Sumtype_Enum_3652118842108341957
<'__parametrized_lt, A > where A : '__parametrized_lt, <
__SumType_RefType_14603428086114329038_0 as
__Sumtype_TypeRef_Trait_3652118842108341957 < '__parametrized_lt, A > > ::
Type : :: core :: iter :: Iterator < Item = __SumType_AssocType_Item >, <
__SumType_RefType_1820574786928708003_1 as
__Sumtype_TypeRef_Trait_3652118842108341957 < '__parametrized_lt, A > > ::
Type : :: core :: iter :: Iterator < Item = __SumType_AssocType_Item >,
{
    type Item = __SumType_AssocType_Item; fn next(& mut self) -> :: core ::
    option :: Option < Self :: Item >
    {
        match self
        {
            __Sumtype_Enum_3652118842108341957
            ::__SumType_Variant_0(__sumtrait_self_arg) => <<
            __SumType_RefType_14603428086114329038_0 as
            __Sumtype_TypeRef_Trait_3652118842108341957 < '__parametrized_lt,
            A > > :: Type as :: core :: iter :: Iterator <
            >>::next(__sumtrait_self_arg), __Sumtype_Enum_3652118842108341957
            ::__SumType_Variant_1(__sumtrait_self_arg) => <<
            __SumType_RefType_1820574786928708003_1 as
            __Sumtype_TypeRef_Trait_3652118842108341957 < '__parametrized_lt,
            A > > :: Type as :: core :: iter :: Iterator <
            >>::next(__sumtrait_self_arg), Self :: __Uninhabited(_) => :: core
            :: unreachable! (),
        }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_impl_trait"
            && e.input == r#"[map, into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self = Vec<T>,
            self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }
            {
                f = f, T = M, Mapped = Vec<M>, param_map =
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() },
            }"#
            && e.to == r#"impl < T, M > ParametrizedMap < 0, M > for Vec<T>
            {
                type Mapped = Vec<M>; fn
                param_map(self, f : impl FnMut(Self :: Item) -> M) -> Self :: Mapped
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() }
            } emit_impl_trait!
            ([into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self = Vec<T>, self =
            self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            });"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "impl_all"
            && e.input == r#"[T] map, into_iter, iter_mut for Vec<T>, T = M, Mapped = Vec<M>; [T] into_iter
            for std::collections::BTreeSet<T>; [T] into_iter for
            std::collections::HashSet<T>; [T] into_iter for
            std::collections::BinaryHeap<T>; [T] map, into_iter, iter_mut for
            std::collections::LinkedList<T>, T = M, Mapped =
            std::collections::LinkedList<M>; [T] map, into_iter, iter_mut for
            std::collections::VecDeque<T>, T = M, Mapped = std::collections::VecDeque<M>;"#
            && e.to == r#"emit_impl_trait!
            ([map, into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self = Vec<T>,
            self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }
            {
                f = f, T = M, Mapped = Vec<M>, param_map =
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() },
            }); emit_impl_trait!
            ([into_iter,] impl_generics = [T], PARAM = 0, Self =
            std::collections::BTreeSet<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }); emit_impl_trait!
            ([into_iter,] impl_generics = [T], PARAM = 0, Self =
            std::collections::HashSet<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }); emit_impl_trait!
            ([into_iter,] impl_generics = [T], PARAM = 0, Self =
            std::collections::BinaryHeap<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }); emit_impl_trait!
            ([map, into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self =
            std::collections::LinkedList<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }
            {
                f = f, T = M, Mapped = std::collections::LinkedList<M>, param_map =
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() },
            }); emit_impl_trait!
            ([map, into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self =
            std::collections::VecDeque<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }
            {
                f = f, T = M, Mapped = std::collections::VecDeque<M>, param_map =
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() },
            });"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "impl_for_tuple"
            && e.input == r#"[] T []"#
            && e.to == r#"impl < T > Parametrized < {impl_for_tuple! (@ count)}> for (T,)
            {
                type Item = T; const MIN_LEN : usize = 1; const MAX_LEN : Option < usize >
                = Some(1); fn param_len(& self) -> usize { 1 } type Iter < 'a > = :: core
                :: iter :: Once < & 'a Self :: Item > where (Self, Self :: Item): 'a; fn
                param_iter < 'a > (& 'a self) -> Self :: Iter < 'a > where Self :: Item :
                'a
                {
                    :: core :: iter ::
                    once(impl_for_tuple!
                    (@ nth []
                    [& self.0, & self.1, & self.2, & self.3, & self.4, & self.5, & self.6,
                    & self.7, & self.8, & self.9, & self.10, & self.11]))
                }
            } impl < T > ParametrizedIterMut < {impl_for_tuple! (@ count)}> for (T,)
            {
                type IterMut < 'a > = :: core :: iter :: Once < & 'a mut Self :: Item >
                where (Self, Self :: Item): 'a; fn param_iter_mut < 'a > (& 'a mut self)
                -> Self :: IterMut < 'a > where Self :: Item : 'a
                {
                    core :: iter ::
                    once(impl_for_tuple!
                    (@ nth []
                    [& mut self.0, & mut self.1, & mut self.2, & mut self.3, & mut self.4,
                    & mut self.5, & mut self.6, & mut self.7, & mut self.8, & mut self.9,
                    & mut self.10, & mut self.11]))
                }
            } impl < T > ParametrizedIntoIter < {impl_for_tuple! (@ count)}> for (T,)
            {
                type IntoIter = :: core :: iter :: Once < Self :: Item > where Self ::
                Item : Sized; fn param_into_iter(self) -> Self :: IntoIter where Self ::
                Item : Sized
                {
                    core :: iter ::
                    once(impl_for_tuple!
                    (@ nth []
                    [self.0, self.1, self.2, self.3, self.4, self.5, self.6, self.7,
                    self.8, self.9, self.10, self.11]))
                }
            } impl < U, T > ParametrizedMap < {impl_for_tuple! (@ count)}, U > for (T,)
            {
                type Mapped = (U,); fn
                param_map(self, mut f : impl FnMut(Self :: Item) -> U) -> Self :: Mapped
                where Self :: Item : Sized
                {
                    impl_for_tuple!
                    (@ wrap_f f [] T []
                    [self.0, self.1, self.2, self.3, self.4, self.5, self.6, self.7,
                    self.8, self.9, self.10, self.11] {})
                }
            }"#
    }));
}

#[test]
fn external_crate_parametrized_test_recursive() {
    let expansions = run_trace_for_repo("parametrized", Some("recursive"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "parametrized"
            && e.input == r#"pub enum E<T> { E0(S<T>), E1, }"#
            && e.to == r#"pub enum E < T > { E0(S < T >), E1, }
#[:: parametrized :: _imp :: sumtype ::
sumtype(:: parametrized :: _imp :: sumtype :: traits :: Iterator)] impl < T >
:: parametrized :: Parametrized <0usize > for E < T >
{
    type Item = T; const MIN_LEN : usize =
    {
        const fn __parametric_type_min(a : usize, b : usize) -> usize
        { if a < b { a } else { b } }
        __parametric_type_min(< S < T > as :: parametrized :: Parametrized <
        0usize > > :: MIN_LEN * 1usize, 0usize)
    }; const MAX_LEN : Option < usize > =
    {
        const fn
        __parametric_type_max(a : Option < usize > , b : Option < usize >) ->
        Option < usize >
        {
            match (a, b)
            {
                (Some(a), Some(b)) => if a > b { Some(a) } else { Some(b) } _
                => None,
            }
        }
        __parametric_type_max(if let (Some(l), Some(r)) =
        (< S < T > as :: parametrized :: Parametrized < 0usize > > :: MAX_LEN,
        :: core :: option :: Option :: Some(1usize)) { Some(l * r) } else
        { None }, :: core :: option :: Option :: Some(0usize))
    }; fn param_len(& self) -> usize
    {
        #[allow(unused)] match self
        {
            E ::E0(__parametric_type_id_0) =>
            {
                < S < T > as :: parametrized :: Parametrized < 0usize > > ::
                param_iter(__parametric_type_id_0).map(| __parametrized_arg |
                1usize).sum :: < :: core :: primitive :: usize > ()
            } E ::E1 => { 0usize }
        }
    } type Iter <'__parametrized_lt > = sumtype! ['__parametrized_lt] where
    (Self, Self :: Item) : '__parametrized_lt; fn param_iter <
    '__parametrized_lt > (& '__parametrized_lt self) -> Self :: Iter <
    '__parametrized_lt > where Self :: Item : '__parametrized_lt
    {
        #[allow(unused)] match self
        {
            E ::E0(__parametric_type_id_0) =>
            {
                sumtype!
                ({
                    let __parametrized_fn : fn(& '__parametrized_lt T) -> _ = |
                    __parametrized_arg |
                    { :: core :: iter :: once(__parametrized_arg) }; ::
                    parametrized :: Flatten ::
                    new(< S < T > as :: parametrized :: Parametrized < 0usize >
                    > ::
                    param_iter(__parametric_type_id_0).map(__parametrized_fn))
                }, for <'__parametrized_lt > :: parametrized :: Flatten < ::
                core :: iter :: Map < < S < T > as :: parametrized ::
                Parametrized < 0usize > > :: Iter < '__parametrized_lt > ,
                fn(& '__parametrized_lt T) -> :: core :: iter :: Once < &
                '__parametrized_lt T > > , :: core :: iter :: Once < &
                '__parametrized_lt T > > where T : '__parametrized_lt,)
            } E ::E1 =>
            {
                sumtype!
                (:: core :: iter :: empty(), for <'__parametrized_lt > :: core
                :: iter :: Empty < & '__parametrized_lt T > where T :
                '__parametrized_lt,)
            }
        }
    }
}
#[:: parametrized :: _imp :: sumtype ::
sumtype(:: parametrized :: _imp :: sumtype :: traits :: Iterator)] impl < T >
:: parametrized :: ParametrizedIntoIter <0usize > for E < T >
{
    type IntoIter = sumtype! []; fn param_into_iter(self) -> Self :: IntoIter
    {
        #[allow(unused)] match self
        {
            E ::E0(__parametric_type_id_0) =>
            {
                sumtype!
                ({
                    let __parametrized_fn : fn(T) -> _ = | __parametrized_arg |
                    { :: core :: iter :: once(__parametrized_arg) }; ::
                    parametrized :: Flatten ::
                    new(< S < T > as :: parametrized :: ParametrizedIntoIter <
                    0usize > > ::
                    param_into_iter(__parametric_type_id_0).map(__parametrized_fn))
                }, :: parametrized :: Flatten < :: core :: iter :: Map < < S <
                T > as :: parametrized :: ParametrizedIntoIter < 0usize > > ::
                IntoIter, fn(T) -> :: core :: iter :: Once < T > > , :: core
                :: iter :: Once < T > >)
            } E ::E1 =>
            {
                sumtype!
                (:: core :: iter :: empty(), :: core :: iter :: Empty < T >)
            }
        }
    }
} impl < T, __PARAMETRIZED_MAP_PARAM > :: parametrized :: ParametrizedMap
<0usize, __PARAMETRIZED_MAP_PARAM > for E < T >
{
    type Mapped = E < __PARAMETRIZED_MAP_PARAM > ; fn
    param_map(self, mut __parametrized_map_fn : impl FnMut(Self :: Item) ->
    __PARAMETRIZED_MAP_PARAM) -> Self :: Mapped where Self :: Item : :: core
    :: marker :: Sized
    {
        match self
        {
            E ::E0(__parametric_type_id_0) =>
            {
                E
                ::E0(< S < T > as :: parametrized :: ParametrizedMap < 0usize,
                __PARAMETRIZED_MAP_PARAM > > ::
                param_map(__parametric_type_id_0, | __parametrized_arg |
                { __parametrized_map_fn(__parametrized_arg) }))
            } E ::E1 => { E ::E1 }
        }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && e.input == r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#
            && e.to == r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_11177185773735460263
{
    ($ ($t : tt) *) =>
    {
        $crate :: _sumtrait_internal!
        ({ $ ($t) * } [],
        {
            #[doc =
            " Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
            #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8])
                -> :: std :: io :: Result < :: core :: primitive :: usize > ;
            }
        }, 10791577178322682870usize, $crate, $crate :: traits :: Marker,
        [:: std :: io :: Read], [], [],);
    };
} #[doc(hidden)] pub use __sumtype_macro_11177185773735460263 as Read;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtype"
            && e.input == r#"impl < T > :: parametrized :: Parametrized <0usize > for E < T >
{
    type Item = T; const MIN_LEN : usize =
    {
        const fn __parametric_type_min(a : usize, b : usize) -> usize
        { if a < b { a } else { b } }
        __parametric_type_min(< S < T > as :: parametrized :: Parametrized <
        0usize > > :: MIN_LEN * 1usize, 0usize)
    }; const MAX_LEN : Option < usize > =
    {
        const fn
        __parametric_type_max(a : Option < usize > , b : Option < usize >) ->
        Option < usize >
        {
            match (a, b)
            {
                (Some(a), Some(b)) => if a > b { Some(a) } else { Some(b) } _
                => None,
            }
        }
        __parametric_type_max(if let (Some(l), Some(r)) =
        (< S < T > as :: parametrized :: Parametrized < 0usize > > :: MAX_LEN,
        :: core :: option :: Option :: Some(1usize)) { Some(l * r) } else
        { None }, :: core :: option :: Option :: Some(0usize))
    }; fn param_len(& self) -> usize
    {
        #[allow(unused)] match self
        {
            E ::E0(__parametric_type_id_0) =>
            {
                < S < T > as :: parametrized :: Parametrized < 0usize > > ::
                param_iter(__parametric_type_id_0).map(| __parametrized_arg |
                1usize).sum :: < :: core :: primitive :: usize > ()
            } E ::E1 => { 0usize }
        }
    } type Iter <'__parametrized_lt > = sumtype! ['__parametrized_lt] where
    (Self, Self :: Item) : '__parametrized_lt; fn param_iter <
    '__parametrized_lt > (& '__parametrized_lt self) -> Self :: Iter <
    '__parametrized_lt > where Self :: Item : '__parametrized_lt
    {
        #[allow(unused)] match self
        {
            E ::E0(__parametric_type_id_0) =>
            {
                sumtype!
                ({
                    let __parametrized_fn : fn(& '__parametrized_lt T) -> _ = |
                    __parametrized_arg |
                    { :: core :: iter :: once(__parametrized_arg) }; ::
                    parametrized :: Flatten ::
                    new(< S < T > as :: parametrized :: Parametrized < 0usize >
                    > ::
                    param_iter(__parametric_type_id_0).map(__parametrized_fn))
                }, for <'__parametrized_lt > :: parametrized :: Flatten < ::
                core :: iter :: Map < < S < T > as :: parametrized ::
                Parametrized < 0usize > > :: Iter < '__parametrized_lt > ,
                fn(& '__parametrized_lt T) -> :: core :: iter :: Once < &
                '__parametrized_lt T > > , :: core :: iter :: Once < &
                '__parametrized_lt T > > where T : '__parametrized_lt,)
            } E ::E1 =>
            {
                sumtype!
                (:: core :: iter :: empty(), for <'__parametrized_lt > :: core
                :: iter :: Empty < & '__parametrized_lt T > where T :
                '__parametrized_lt,)
            }
        }
    }
}"#
            && e.to == r#"#[doc(hidden)] #[allow(non_camel_case_types)] #[allow(non_camel_case_types)]
struct __SumType_RefType_17755882455004361104_0; #[doc(hidden)]
#[allow(non_camel_case_types)] #[allow(non_camel_case_types)] struct
__SumType_RefType_6921341630406533884_1; #[doc(hidden)]
#[allow(non_camel_case_types)] trait
__Sumtype_TypeRef_Trait_13365136640141107033 <'__parametrized_lt, T :
'__parametrized_lt > { type Type; } #[doc(hidden)]
#[allow(non_camel_case_types)] pub enum __Sumtype_Enum_13365136640141107033 <
'__parametrized_lt, T : '__parametrized_lt >
{
    __SumType_Variant_0(< __SumType_RefType_17755882455004361104_0 as
    __Sumtype_TypeRef_Trait_13365136640141107033 < '__parametrized_lt, T > >
    :: Type),
    __SumType_Variant_1(< __SumType_RefType_6921341630406533884_1 as
    __Sumtype_TypeRef_Trait_13365136640141107033 < '__parametrized_lt, T > >
    :: Type),
    __Uninhabited((:: core :: convert :: Infallible, :: core :: marker ::
    PhantomData <& '__parametrized_lt () > , :: core :: marker :: PhantomData
    <T >)),
} #[doc(hidden)] #[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_13365136640141107033 <'__parametrized_lt, T :
'__parametrized_lt > {} impl <'__parametrized_lt, T : '__parametrized_lt,
__Sumtype_TypeParam > __Sumtype_ConstraintExprTrait_13365136640141107033
<'__parametrized_lt, T > for __Sumtype_TypeParam where __Sumtype_TypeParam :
__Sumtype_ConstraintExprTrait_0_8650592876115685186 <'__parametrized_lt, T >,
T : '__parametrized_lt, {} :: parametrized :: _imp :: sumtype :: traits ::
Iterator!
(__Sumtype_ConstraintExprTrait_0_8650592876115685186, :: parametrized :: _imp
:: sumtype :: traits :: Iterator, __Sumtype_Enum_13365136640141107033, [],
[__SumType_Variant_0 :< __SumType_RefType_17755882455004361104_0 as
__Sumtype_TypeRef_Trait_13365136640141107033 < '__parametrized_lt, T > > ::
Type, __SumType_Variant_1 :< __SumType_RefType_6921341630406533884_1 as
__Sumtype_TypeRef_Trait_13365136640141107033 < '__parametrized_lt, T > > ::
Type], ['__parametrized_lt, T : '__parametrized_lt], ['__parametrized_lt, T],
{ T : '__parametrized_lt },); #[allow(non_local_definitions)] impl < T > ::
parametrized :: Parametrized < 0usize > for E < T >
{
    type Item = T; const MIN_LEN : usize =
    {
        const fn __parametric_type_min(a : usize, b : usize) -> usize
        { if a < b { a } else { b } }
        __parametric_type_min(< S < T > as :: parametrized :: Parametrized <
        0usize > > :: MIN_LEN * 1usize, 0usize)
    }; const MAX_LEN : Option < usize > =
    {
        const fn
        __parametric_type_max(a : Option < usize > , b : Option < usize >) ->
        Option < usize >
        {
            match (a, b)
            {
                (Some(a), Some(b)) => if a > b { Some(a) } else { Some(b) } _
                => None,
            }
        }
        __parametric_type_max(if let (Some(l), Some(r)) =
        (< S < T > as :: parametrized :: Parametrized < 0usize > > :: MAX_LEN,
        :: core :: option :: Option :: Some(1usize)) { Some(l * r) } else
        { None }, :: core :: option :: Option :: Some(0usize))
    }; fn param_len(& self) -> usize
    {
        #[allow(unused)] match self
        {
            E :: E0(__parametric_type_id_0) =>
            {
                < S < T > as :: parametrized :: Parametrized < 0usize > > ::
                param_iter(__parametric_type_id_0).map(| __parametrized_arg |
                1usize).sum :: < :: core :: primitive :: usize > ()
            } E :: E1 => { 0usize }
        }
    } type Iter < '__parametrized_lt > = __Sumtype_Enum_13365136640141107033 <
    '__parametrized_lt, T > where (Self, Self :: Item) : '__parametrized_lt;
    fn param_iter < '__parametrized_lt > (& '__parametrized_lt self) -> Self
    :: Iter < '__parametrized_lt > where Self :: Item : '__parametrized_lt
    {
        #[allow(unused)] match self
        {
            E :: E0(__parametric_type_id_0) =>
            {
                {
                    impl < '__parametrized_lt, T, >
                    __Sumtype_TypeRef_Trait_13365136640141107033 <
                    '__parametrized_lt, T > for
                    __SumType_RefType_17755882455004361104_0 where T :
                    '__parametrized_lt,
                    {
                        type Type = :: parametrized :: Flatten < :: core :: iter ::
                        Map < < S < T > as :: parametrized :: Parametrized < 0usize
                        > > :: Iter < '__parametrized_lt > ,
                        fn(& '__parametrized_lt T) -> :: core :: iter :: Once < &
                        '__parametrized_lt T > > , :: core :: iter :: Once < &
                        '__parametrized_lt T > > ;
                    } fn __sum_type_id_fn_10876125093757221771 <
                    '__parametrized_lt, T, __SumType_T :
                    __Sumtype_ConstraintExprTrait_13365136640141107033 <
                    '__parametrized_lt, T > > (t : __SumType_T) -> __SumType_T
                    where T : '__parametrized_lt, { t }
                    __sum_type_id_fn_10876125093757221771 :: <
                    '__parametrized_lt, T, _ >
                    (__Sumtype_Enum_13365136640141107033 ::
                    __SumType_Variant_0({
                        let __parametrized_fn : fn(& '__parametrized_lt T) -> _ = |
                        __parametrized_arg |
                        { :: core :: iter :: once(__parametrized_arg) }; ::
                        parametrized :: Flatten ::
                        new(< S < T > as :: parametrized :: Parametrized < 0usize >
                        > ::
                        param_iter(__parametric_type_id_0).map(__parametrized_fn))
                    }))
                }
            } E :: E1 =>
            {
                {
                    impl < '__parametrized_lt, T, >
                    __Sumtype_TypeRef_Trait_13365136640141107033 <
                    '__parametrized_lt, T > for
                    __SumType_RefType_6921341630406533884_1 where T :
                    '__parametrized_lt,
                    {
                        type Type = :: core :: iter :: Empty < & '__parametrized_lt
                        T > ;
                    } fn __sum_type_id_fn_6659976167671584328 <
                    '__parametrized_lt, T, __SumType_T :
                    __Sumtype_ConstraintExprTrait_13365136640141107033 <
                    '__parametrized_lt, T > > (t : __SumType_T) -> __SumType_T
                    where T : '__parametrized_lt, { t }
                    __sum_type_id_fn_6659976167671584328 :: <
                    '__parametrized_lt, T, _ >
                    (__Sumtype_Enum_13365136640141107033 ::
                    __SumType_Variant_1(:: core :: iter :: empty()))
                }
            }
        }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__sumtype_macro_5221274140601386781"
            && e.input == r#"__Sumtype_ConstraintExprTrait_0_8650592876115685186, :: parametrized :: _imp
          :: sumtype :: traits :: Iterator, __Sumtype_Enum_13365136640141107033, [],
          [__SumType_Variant_0 :< __SumType_RefType_17755882455004361104_0 as
          __Sumtype_TypeRef_Trait_13365136640141107033 < '__parametrized_lt, T > > ::
          Type, __SumType_Variant_1 :< __SumType_RefType_6921341630406533884_1 as
          __Sumtype_TypeRef_Trait_13365136640141107033 < '__parametrized_lt, T > > ::
          Type], ['__parametrized_lt, T : '__parametrized_lt], ['__parametrized_lt, T],
          { T : '__parametrized_lt },"#
            && e.to == r#"$crate :: _sumtrait_internal!
          ({
              __Sumtype_ConstraintExprTrait_0_8650592876115685186, :: parametrized ::
              _imp :: sumtype :: traits :: Iterator,
              __Sumtype_Enum_13365136640141107033, [],
              [__SumType_Variant_0 :< __SumType_RefType_17755882455004361104_0 as
              __Sumtype_TypeRef_Trait_13365136640141107033 < '__parametrized_lt, T > >
              :: Type, __SumType_Variant_1 :< __SumType_RefType_6921341630406533884_1 as
              __Sumtype_TypeRef_Trait_13365136640141107033 < '__parametrized_lt, T > >
              :: Type], ['__parametrized_lt, T : '__parametrized_lt],
              ['__parametrized_lt, T], { T : '__parametrized_lt },
          } [],
          {
              #[doc =
              " Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`]."]
              #[allow(private_bounds)] pub trait Iterator
              {
                  type Item; fn next(& mut self) -> :: core :: option :: Option < Self
                  :: Item > ;
              }
          }, 15686212630352170898usize, $crate, $crate :: traits :: Marker,
          [:: core :: iter :: Iterator], [], [],);"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "_sumtrait_internal"
            && e.input == r#"{
    __Sumtype_ConstraintExprTrait_0_8650592876115685186, :: parametrized ::
    _imp :: sumtype :: traits :: Iterator,
    __Sumtype_Enum_13365136640141107033, [],
    [__SumType_Variant_0 :< __SumType_RefType_17755882455004361104_0 as
    __Sumtype_TypeRef_Trait_13365136640141107033 < '__parametrized_lt, T > >
    :: Type, __SumType_Variant_1 :< __SumType_RefType_6921341630406533884_1 as
    __Sumtype_TypeRef_Trait_13365136640141107033 < '__parametrized_lt, T > >
    :: Type], ['__parametrized_lt, T : '__parametrized_lt],
    ['__parametrized_lt, T], { T : '__parametrized_lt },
} [],
{
    #[doc =
    " Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`]."]
    #[allow(private_bounds)] pub trait Iterator
    {
        type Item; fn next(& mut self) -> :: core :: option :: Option < Self
        :: Item > ;
    }
}, 15686212630352170898usize, $crate, $crate :: traits :: Marker,
[:: core :: iter :: Iterator], [], [],"#
            && e.to == r#"#[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_0_8650592876115685186 <'__parametrized_lt, T :
'__parametrized_lt > {} impl <'__parametrized_lt, T : '__parametrized_lt,
__SumType_AssocType_Item > __Sumtype_ConstraintExprTrait_0_8650592876115685186
<'__parametrized_lt, T > for __Sumtype_Enum_13365136640141107033
<'__parametrized_lt, T > where T : '__parametrized_lt, <
__SumType_RefType_17755882455004361104_0 as
__Sumtype_TypeRef_Trait_13365136640141107033 < '__parametrized_lt, T > > ::
Type : :: core :: iter :: Iterator < Item = __SumType_AssocType_Item >, <
__SumType_RefType_6921341630406533884_1 as
__Sumtype_TypeRef_Trait_13365136640141107033 < '__parametrized_lt, T > > ::
Type : :: core :: iter :: Iterator < Item = __SumType_AssocType_Item >, {}
impl <'__parametrized_lt, T : '__parametrized_lt, __SumType_AssocType_Item >
:: core :: iter :: Iterator <> for __Sumtype_Enum_13365136640141107033
<'__parametrized_lt, T > where T : '__parametrized_lt, <
__SumType_RefType_17755882455004361104_0 as
__Sumtype_TypeRef_Trait_13365136640141107033 < '__parametrized_lt, T > > ::
Type : :: core :: iter :: Iterator < Item = __SumType_AssocType_Item >, <
__SumType_RefType_6921341630406533884_1 as
__Sumtype_TypeRef_Trait_13365136640141107033 < '__parametrized_lt, T > > ::
Type : :: core :: iter :: Iterator < Item = __SumType_AssocType_Item >,
{
    type Item = __SumType_AssocType_Item; fn next(& mut self) -> :: core ::
    option :: Option < Self :: Item >
    {
        match self
        {
            __Sumtype_Enum_13365136640141107033
            ::__SumType_Variant_0(__sumtrait_self_arg) => <<
            __SumType_RefType_17755882455004361104_0 as
            __Sumtype_TypeRef_Trait_13365136640141107033 < '__parametrized_lt,
            T > > :: Type as :: core :: iter :: Iterator <
            >>::next(__sumtrait_self_arg), __Sumtype_Enum_13365136640141107033
            ::__SumType_Variant_1(__sumtrait_self_arg) => <<
            __SumType_RefType_6921341630406533884_1 as
            __Sumtype_TypeRef_Trait_13365136640141107033 < '__parametrized_lt,
            T > > :: Type as :: core :: iter :: Iterator <
            >>::next(__sumtrait_self_arg), Self :: __Uninhabited(_) => :: core
            :: unreachable! (),
        }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_impl_trait"
            && e.input == r#"[map, into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self = Vec<T>,
            self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }
            {
                f = f, T = M, Mapped = Vec<M>, param_map =
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() },
            }"#
            && e.to == r#"impl < T, M > ParametrizedMap < 0, M > for Vec<T>
            {
                type Mapped = Vec<M>; fn
                param_map(self, f : impl FnMut(Self :: Item) -> M) -> Self :: Mapped
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() }
            } emit_impl_trait!
            ([into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self = Vec<T>, self =
            self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            });"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "impl_all"
            && e.input == r#"[T] map, into_iter, iter_mut for Vec<T>, T = M, Mapped = Vec<M>; [T] into_iter
            for std::collections::BTreeSet<T>; [T] into_iter for
            std::collections::HashSet<T>; [T] into_iter for
            std::collections::BinaryHeap<T>; [T] map, into_iter, iter_mut for
            std::collections::LinkedList<T>, T = M, Mapped =
            std::collections::LinkedList<M>; [T] map, into_iter, iter_mut for
            std::collections::VecDeque<T>, T = M, Mapped = std::collections::VecDeque<M>;"#
            && e.to == r#"emit_impl_trait!
            ([map, into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self = Vec<T>,
            self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }
            {
                f = f, T = M, Mapped = Vec<M>, param_map =
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() },
            }); emit_impl_trait!
            ([into_iter,] impl_generics = [T], PARAM = 0, Self =
            std::collections::BTreeSet<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }); emit_impl_trait!
            ([into_iter,] impl_generics = [T], PARAM = 0, Self =
            std::collections::HashSet<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }); emit_impl_trait!
            ([into_iter,] impl_generics = [T], PARAM = 0, Self =
            std::collections::BinaryHeap<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }); emit_impl_trait!
            ([map, into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self =
            std::collections::LinkedList<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }
            {
                f = f, T = M, Mapped = std::collections::LinkedList<M>, param_map =
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() },
            }); emit_impl_trait!
            ([map, into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self =
            std::collections::VecDeque<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }
            {
                f = f, T = M, Mapped = std::collections::VecDeque<M>, param_map =
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() },
            });"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "impl_for_tuple"
            && e.input == r#"[] T []"#
            && e.to == r#"impl < T > Parametrized < {impl_for_tuple! (@ count)}> for (T,)
            {
                type Item = T; const MIN_LEN : usize = 1; const MAX_LEN : Option < usize >
                = Some(1); fn param_len(& self) -> usize { 1 } type Iter < 'a > = :: core
                :: iter :: Once < & 'a Self :: Item > where (Self, Self :: Item): 'a; fn
                param_iter < 'a > (& 'a self) -> Self :: Iter < 'a > where Self :: Item :
                'a
                {
                    :: core :: iter ::
                    once(impl_for_tuple!
                    (@ nth []
                    [& self.0, & self.1, & self.2, & self.3, & self.4, & self.5, & self.6,
                    & self.7, & self.8, & self.9, & self.10, & self.11]))
                }
            } impl < T > ParametrizedIterMut < {impl_for_tuple! (@ count)}> for (T,)
            {
                type IterMut < 'a > = :: core :: iter :: Once < & 'a mut Self :: Item >
                where (Self, Self :: Item): 'a; fn param_iter_mut < 'a > (& 'a mut self)
                -> Self :: IterMut < 'a > where Self :: Item : 'a
                {
                    core :: iter ::
                    once(impl_for_tuple!
                    (@ nth []
                    [& mut self.0, & mut self.1, & mut self.2, & mut self.3, & mut self.4,
                    & mut self.5, & mut self.6, & mut self.7, & mut self.8, & mut self.9,
                    & mut self.10, & mut self.11]))
                }
            } impl < T > ParametrizedIntoIter < {impl_for_tuple! (@ count)}> for (T,)
            {
                type IntoIter = :: core :: iter :: Once < Self :: Item > where Self ::
                Item : Sized; fn param_into_iter(self) -> Self :: IntoIter where Self ::
                Item : Sized
                {
                    core :: iter ::
                    once(impl_for_tuple!
                    (@ nth []
                    [self.0, self.1, self.2, self.3, self.4, self.5, self.6, self.7,
                    self.8, self.9, self.10, self.11]))
                }
            } impl < U, T > ParametrizedMap < {impl_for_tuple! (@ count)}, U > for (T,)
            {
                type Mapped = (U,); fn
                param_map(self, mut f : impl FnMut(Self :: Item) -> U) -> Self :: Mapped
                where Self :: Item : Sized
                {
                    impl_for_tuple!
                    (@ wrap_f f [] T []
                    [self.0, self.1, self.2, self.3, self.4, self.5, self.6, self.7,
                    self.8, self.9, self.10, self.11] {})
                }
            }"#
    }));
}

#[test]
fn external_crate_parametrized_test_test() {
    let expansions = run_trace_for_repo("parametrized", Some("test"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "parametrized"
            && e.input == r#"struct Struct1<K>(usize, Vec<(usize, K)>);"#
            && e.to == r#"struct Struct1 < K > (usize, Vec < (usize, K) >); impl < K > :: parametrized
:: ParametrizedIntoIter <0usize > for Struct1 < K >
{
    type IntoIter = :: parametrized :: Flatten < :: core :: iter :: Map < <
    Vec < (usize, K) > as :: parametrized :: ParametrizedIntoIter < 0usize > >
    :: IntoIter, fn((usize, K)) -> :: core :: iter :: Once < K > > , :: core
    :: iter :: Once < K > > ; fn param_into_iter(self) -> Self :: IntoIter
    {
        {
            let __parametrized_fn : fn((usize, K)) -> _ = | __parametrized_arg
            | { :: core :: iter :: once(__parametrized_arg.1) }; ::
            parametrized :: Flatten ::
            new(< Vec < (usize, K) > as :: parametrized ::
            ParametrizedIntoIter < 0usize > > ::
            param_into_iter(self.1).map(__parametrized_fn))
        }
    }
} impl < K > :: parametrized :: Parametrized <0usize > for Struct1 < K >
{
    type Item = K; const MIN_LEN : usize =
    {
        const fn __parametric_type_min(a : usize, b : usize) -> usize
        { if a < b { a } else { b } } < Vec < (usize, K) > as :: parametrized
        :: Parametrized < 0usize > > :: MIN_LEN * < (usize, K) as ::
        parametrized :: Parametrized < 1usize > > :: MIN_LEN * 1usize
    }; const MAX_LEN : Option < usize > =
    {
        const fn
        __parametric_type_max(a : Option < usize > , b : Option < usize >) ->
        Option < usize >
        {
            match (a, b)
            {
                (Some(a), Some(b)) => if a > b { Some(a) } else { Some(b) } _
                => None,
            }
        } if let (Some(l), Some(r)) =
        (< Vec < (usize, K) > as :: parametrized :: Parametrized < 0usize > >
        :: MAX_LEN, if let (Some(l), Some(r)) =
        (< (usize, K) as :: parametrized :: Parametrized < 1usize > > ::
        MAX_LEN, :: core :: option :: Option :: Some(1usize)) { Some(l * r) }
        else { None }) { Some(l * r) } else { None }
    }; fn param_len(& self) -> usize
    {
        < Vec < (usize, K) > as :: parametrized :: Parametrized < 0usize > >
        ::
        param_iter(&
        self.1).map(| __parametrized_arg | < (usize, K) as :: parametrized ::
        Parametrized < 1usize > > ::
        param_iter(__parametrized_arg).map(| __parametrized_arg | 1usize).sum
        :: < :: core :: primitive :: usize > ()).sum :: < :: core :: primitive
        :: usize > ()
    } type Iter <'__parametrized_lt > = :: parametrized :: Flatten < :: core
    :: iter :: Map < < Vec < (usize, K) > as :: parametrized :: Parametrized <
    0usize > > :: Iter < '__parametrized_lt > ,
    fn(& '__parametrized_lt (usize, K)) -> :: parametrized :: Flatten < ::
    core :: iter :: Map < < (usize, K) as :: parametrized :: Parametrized <
    1usize > > :: Iter < '__parametrized_lt > , fn(& '__parametrized_lt K) ->
    :: core :: iter :: Once < & '__parametrized_lt K > > , :: core :: iter ::
    Once < & '__parametrized_lt K > > > , :: parametrized :: Flatten < :: core
    :: iter :: Map < < (usize, K) as :: parametrized :: Parametrized < 1usize
    > > :: Iter < '__parametrized_lt > , fn(& '__parametrized_lt K) -> :: core
    :: iter :: Once < & '__parametrized_lt K > > , :: core :: iter :: Once < &
    '__parametrized_lt K > > > where (Self, Self :: Item) :
    '__parametrized_lt; fn param_iter < '__parametrized_lt >
    (& '__parametrized_lt self) -> Self :: Iter < '__parametrized_lt > where
    Self :: Item : '__parametrized_lt
    {
        {
            let __parametrized_fn : fn(& '__parametrized_lt (usize, K)) -> _ =
            | __parametrized_arg |
            {
                {
                    let __parametrized_fn : fn(& '__parametrized_lt K) -> _ = |
                    __parametrized_arg |
                    { :: core :: iter :: once(__parametrized_arg) }; ::
                    parametrized :: Flatten ::
                    new(< (usize, K) as :: parametrized :: Parametrized < 1usize
                    > > ::
                    param_iter(__parametrized_arg).map(__parametrized_fn))
                }
            }; :: parametrized :: Flatten ::
            new(< Vec < (usize, K) > as :: parametrized :: Parametrized <
            0usize > > :: param_iter(& self.1).map(__parametrized_fn))
        }
    }
} impl < K, __PARAMETRIZED_MAP_PARAM > :: parametrized :: ParametrizedMap
<0usize, __PARAMETRIZED_MAP_PARAM > for Struct1 < K >
{
    type Mapped = Struct1 < __PARAMETRIZED_MAP_PARAM > ; fn
    param_map(self, mut __parametrized_map_fn : impl FnMut(Self :: Item) ->
    __PARAMETRIZED_MAP_PARAM) -> Self :: Mapped where Self :: Item : :: core
    :: marker :: Sized
    {
        #[allow(unused)]
        Struct1(self.0, < Vec < (usize, K) > as :: parametrized ::
        ParametrizedMap < 0usize, (usize, __PARAMETRIZED_MAP_PARAM) > > ::
        param_map(self.1, | __parametrized_arg |
        {
            < (usize, K) as :: parametrized :: ParametrizedMap < 1usize,
            __PARAMETRIZED_MAP_PARAM > > ::
            param_map(__parametrized_arg, | __parametrized_arg |
            { __parametrized_map_fn(__parametrized_arg) })
        }))
    }
} impl < K > :: parametrized :: ParametrizedIterMut <0usize > for Struct1 < K
>
{
    type IterMut <'__parametrized_lt > = :: parametrized :: Flatten < :: core
    :: iter :: Map < < Vec < (usize, K) > as :: parametrized ::
    ParametrizedIterMut < 0usize > > :: IterMut < '__parametrized_lt > ,
    fn(& '__parametrized_lt mut (usize, K)) -> :: core :: iter :: Once < &
    '__parametrized_lt mut K > > , :: core :: iter :: Once < &
    '__parametrized_lt mut K > > where (Self, Self :: Item) :
    '__parametrized_lt; fn param_iter_mut < '__parametrized_lt >
    (& '__parametrized_lt mut self) -> Self :: IterMut < '__parametrized_lt >
    where Self :: Item : '__parametrized_lt
    {
        {
            let __parametrized_fn : fn(& '__parametrized_lt mut (usize, K)) ->
            _ = | __parametrized_arg |
            { :: core :: iter :: once(& mut __parametrized_arg.1) }; ::
            parametrized :: Flatten ::
            new(< Vec < (usize, K) > as :: parametrized :: ParametrizedIterMut
            < 0usize > > ::
            param_iter_mut(& mut self.1).map(__parametrized_fn))
        }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && e.input == r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#
            && e.to == r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_11177185773735460263
{
    ($ ($t : tt) *) =>
    {
        $crate :: _sumtrait_internal!
        ({ $ ($t) * } [],
        {
            #[doc =
            " Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
            #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8])
                -> :: std :: io :: Result < :: core :: primitive :: usize > ;
            }
        }, 10791577178322682870usize, $crate, $crate :: traits :: Marker,
        [:: std :: io :: Read], [], [],);
    };
} #[doc(hidden)] pub use __sumtype_macro_11177185773735460263 as Read;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_impl_trait"
            && e.input == r#"[map, into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self = Vec<T>,
            self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }
            {
                f = f, T = M, Mapped = Vec<M>, param_map =
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() },
            }"#
            && e.to == r#"impl < T, M > ParametrizedMap < 0, M > for Vec<T>
            {
                type Mapped = Vec<M>; fn
                param_map(self, f : impl FnMut(Self :: Item) -> M) -> Self :: Mapped
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() }
            } emit_impl_trait!
            ([into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self = Vec<T>, self =
            self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            });"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "impl_all"
            && e.input == r#"[T] map, into_iter, iter_mut for Vec<T>, T = M, Mapped = Vec<M>; [T] into_iter
            for std::collections::BTreeSet<T>; [T] into_iter for
            std::collections::HashSet<T>; [T] into_iter for
            std::collections::BinaryHeap<T>; [T] map, into_iter, iter_mut for
            std::collections::LinkedList<T>, T = M, Mapped =
            std::collections::LinkedList<M>; [T] map, into_iter, iter_mut for
            std::collections::VecDeque<T>, T = M, Mapped = std::collections::VecDeque<M>;"#
            && e.to == r#"emit_impl_trait!
            ([map, into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self = Vec<T>,
            self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }
            {
                f = f, T = M, Mapped = Vec<M>, param_map =
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() },
            }); emit_impl_trait!
            ([into_iter,] impl_generics = [T], PARAM = 0, Self =
            std::collections::BTreeSet<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }); emit_impl_trait!
            ([into_iter,] impl_generics = [T], PARAM = 0, Self =
            std::collections::HashSet<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }); emit_impl_trait!
            ([into_iter,] impl_generics = [T], PARAM = 0, Self =
            std::collections::BinaryHeap<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }); emit_impl_trait!
            ([map, into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self =
            std::collections::LinkedList<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }
            {
                f = f, T = M, Mapped = std::collections::LinkedList<M>, param_map =
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() },
            }); emit_impl_trait!
            ([map, into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self =
            std::collections::VecDeque<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }
            {
                f = f, T = M, Mapped = std::collections::VecDeque<M>, param_map =
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() },
            });"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "impl_for_tuple"
            && e.input == r#"[] T []"#
            && e.to == r#"impl < T > Parametrized < {impl_for_tuple! (@ count)}> for (T,)
            {
                type Item = T; const MIN_LEN : usize = 1; const MAX_LEN : Option < usize >
                = Some(1); fn param_len(& self) -> usize { 1 } type Iter < 'a > = :: core
                :: iter :: Once < & 'a Self :: Item > where (Self, Self :: Item): 'a; fn
                param_iter < 'a > (& 'a self) -> Self :: Iter < 'a > where Self :: Item :
                'a
                {
                    :: core :: iter ::
                    once(impl_for_tuple!
                    (@ nth []
                    [& self.0, & self.1, & self.2, & self.3, & self.4, & self.5, & self.6,
                    & self.7, & self.8, & self.9, & self.10, & self.11]))
                }
            } impl < T > ParametrizedIterMut < {impl_for_tuple! (@ count)}> for (T,)
            {
                type IterMut < 'a > = :: core :: iter :: Once < & 'a mut Self :: Item >
                where (Self, Self :: Item): 'a; fn param_iter_mut < 'a > (& 'a mut self)
                -> Self :: IterMut < 'a > where Self :: Item : 'a
                {
                    core :: iter ::
                    once(impl_for_tuple!
                    (@ nth []
                    [& mut self.0, & mut self.1, & mut self.2, & mut self.3, & mut self.4,
                    & mut self.5, & mut self.6, & mut self.7, & mut self.8, & mut self.9,
                    & mut self.10, & mut self.11]))
                }
            } impl < T > ParametrizedIntoIter < {impl_for_tuple! (@ count)}> for (T,)
            {
                type IntoIter = :: core :: iter :: Once < Self :: Item > where Self ::
                Item : Sized; fn param_into_iter(self) -> Self :: IntoIter where Self ::
                Item : Sized
                {
                    core :: iter ::
                    once(impl_for_tuple!
                    (@ nth []
                    [self.0, self.1, self.2, self.3, self.4, self.5, self.6, self.7,
                    self.8, self.9, self.10, self.11]))
                }
            } impl < U, T > ParametrizedMap < {impl_for_tuple! (@ count)}, U > for (T,)
            {
                type Mapped = (U,); fn
                param_map(self, mut f : impl FnMut(Self :: Item) -> U) -> Self :: Mapped
                where Self :: Item : Sized
                {
                    impl_for_tuple!
                    (@ wrap_f f [] T []
                    [self.0, self.1, self.2, self.3, self.4, self.5, self.6, self.7,
                    self.8, self.9, self.10, self.11] {})
                }
            }"#
    }));
}

#[test]
fn external_crate_parametrized_test_test_enum() {
    let expansions = run_trace_for_repo("parametrized", Some("test_enum"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "parametrized"
            && e.input == r#"enum Enum1<K> { V1, V2(K), V3 { _f1: usize, _f2: K }, }"#
            && e.to == r#"enum Enum1 < K > { V1, V2(K), V3 { _f1 : usize, _f2 : K }, }
#[:: parametrized :: _imp :: sumtype ::
sumtype(:: parametrized :: _imp :: sumtype :: traits :: Iterator)] impl < K >
:: parametrized :: ParametrizedIntoIter <0usize > for Enum1 < K >
{
    type IntoIter = sumtype! []; fn param_into_iter(self) -> Self :: IntoIter
    {
        #[allow(unused)] match self
        {
            Enum1 ::V1 =>
            {
                sumtype!
                (:: core :: iter :: empty(), :: core :: iter :: Empty < K >)
            } Enum1 ::V2(__parametric_type_id_0) =>
            {
                sumtype!
                (:: core :: iter :: once(__parametric_type_id_0), :: core ::
                iter :: Once < K >)
            } Enum1 ::V3 { _f1, _f2 } =>
            {
                sumtype!
                (:: core :: iter :: once(_f2), :: core :: iter :: Once < K >)
            }
        }
    }
}
#[:: parametrized :: _imp :: sumtype ::
sumtype(:: parametrized :: _imp :: sumtype :: traits :: Iterator)] impl < K >
:: parametrized :: Parametrized <0usize > for Enum1 < K >
{
    type Item = K; const MIN_LEN : usize =
    {
        const fn __parametric_type_min(a : usize, b : usize) -> usize
        { if a < b { a } else { b } }
        __parametric_type_min(0usize, __parametric_type_min(1usize, 1usize))
    }; const MAX_LEN : Option < usize > =
    {
        const fn
        __parametric_type_max(a : Option < usize > , b : Option < usize >) ->
        Option < usize >
        {
            match (a, b)
            {
                (Some(a), Some(b)) => if a > b { Some(a) } else { Some(b) } _
                => None,
            }
        }
        __parametric_type_max(:: core :: option :: Option :: Some(0usize),
        __parametric_type_max(:: core :: option :: Option :: Some(1usize), ::
        core :: option :: Option :: Some(1usize)))
    }; fn param_len(& self) -> usize
    {
        #[allow(unused)] match self
        {
            Enum1 ::V1 => { 0usize } Enum1 ::V2(__parametric_type_id_0) =>
            { 1usize } Enum1 ::V3 { _f1, _f2 } => { 1usize }
        }
    } type Iter <'__parametrized_lt > = sumtype! ['__parametrized_lt] where
    (Self, Self :: Item) : '__parametrized_lt; fn param_iter <
    '__parametrized_lt > (& '__parametrized_lt self) -> Self :: Iter <
    '__parametrized_lt > where Self :: Item : '__parametrized_lt
    {
        #[allow(unused)] match self
        {
            Enum1 ::V1 =>
            {
                sumtype!
                (:: core :: iter :: empty(), for <'__parametrized_lt > :: core
                :: iter :: Empty < & '__parametrized_lt K > where K :
                '__parametrized_lt,)
            } Enum1 ::V2(__parametric_type_id_0) =>
            {
                sumtype!
                (:: core :: iter :: once(__parametric_type_id_0), for
                <'__parametrized_lt > :: core :: iter :: Once < &
                '__parametrized_lt K > where K : '__parametrized_lt,)
            } Enum1 ::V3 { _f1, _f2 } =>
            {
                sumtype!
                (:: core :: iter :: once(_f2), for <'__parametrized_lt > ::
                core :: iter :: Once < & '__parametrized_lt K > where K :
                '__parametrized_lt,)
            }
        }
    }
}
#[:: parametrized :: _imp :: sumtype ::
sumtype(:: parametrized :: _imp :: sumtype :: traits :: Iterator)] impl < K >
:: parametrized :: ParametrizedIterMut <0usize > for Enum1 < K >
{
    type IterMut <'__parametrized_lt > = sumtype! ['__parametrized_lt] where
    (Self, Self :: Item) : '__parametrized_lt; fn param_iter_mut <
    '__parametrized_lt > (& '__parametrized_lt mut self) -> Self :: IterMut <
    '__parametrized_lt > where Self :: Item : '__parametrized_lt
    {
        #[allow(unused)] match self
        {
            Enum1 ::V1 =>
            {
                sumtype!
                (:: core :: iter :: empty(), for <'__parametrized_lt > :: core
                :: iter :: Empty < & '__parametrized_lt mut K > where K :
                '__parametrized_lt,)
            } Enum1 ::V2(__parametric_type_id_0) =>
            {
                sumtype!
                (:: core :: iter :: once(__parametric_type_id_0), for
                <'__parametrized_lt > :: core :: iter :: Once < &
                '__parametrized_lt mut K > where K : '__parametrized_lt,)
            } Enum1 ::V3 { _f1, _f2 } =>
            {
                sumtype!
                (:: core :: iter :: once(_f2), for <'__parametrized_lt > ::
                core :: iter :: Once < & '__parametrized_lt mut K > where K :
                '__parametrized_lt,)
            }
        }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && e.input == r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#
            && e.to == r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_11177185773735460263
{
    ($ ($t : tt) *) =>
    {
        $crate :: _sumtrait_internal!
        ({ $ ($t) * } [],
        {
            #[doc =
            " Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
            #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8])
                -> :: std :: io :: Result < :: core :: primitive :: usize > ;
            }
        }, 10791577178322682870usize, $crate, $crate :: traits :: Marker,
        [:: std :: io :: Read], [], [],);
    };
} #[doc(hidden)] pub use __sumtype_macro_11177185773735460263 as Read;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtype"
            && e.input == r#"impl < K > :: parametrized :: ParametrizedIntoIter <0usize > for Enum1 < K >
{
    type IntoIter = sumtype! []; fn param_into_iter(self) -> Self :: IntoIter
    {
        #[allow(unused)] match self
        {
            Enum1 ::V1 =>
            {
                sumtype!
                (:: core :: iter :: empty(), :: core :: iter :: Empty < K >)
            } Enum1 ::V2(__parametric_type_id_0) =>
            {
                sumtype!
                (:: core :: iter :: once(__parametric_type_id_0), :: core ::
                iter :: Once < K >)
            } Enum1 ::V3 { _f1, _f2 } =>
            {
                sumtype!
                (:: core :: iter :: once(_f2), :: core :: iter :: Once < K >)
            }
        }
    }
}"#
            && e.to == r#"#[doc(hidden)] #[allow(non_camel_case_types)] #[allow(non_camel_case_types)]
struct __SumType_RefType_3587124120095538834_0; #[doc(hidden)]
#[allow(non_camel_case_types)] #[allow(non_camel_case_types)] struct
__SumType_RefType_18391022986078541733_1; #[doc(hidden)]
#[allow(non_camel_case_types)] #[allow(non_camel_case_types)] struct
__SumType_RefType_12550977140906565614_2; #[doc(hidden)]
#[allow(non_camel_case_types)] trait
__Sumtype_TypeRef_Trait_14187255806582459617 <K > { type Type; }
#[doc(hidden)] #[allow(non_camel_case_types)] pub enum
__Sumtype_Enum_14187255806582459617 < K >
{
    __SumType_Variant_0(< __SumType_RefType_3587124120095538834_0 as
    __Sumtype_TypeRef_Trait_14187255806582459617 < K > > :: Type),
    __SumType_Variant_1(< __SumType_RefType_18391022986078541733_1 as
    __Sumtype_TypeRef_Trait_14187255806582459617 < K > > :: Type),
    __SumType_Variant_2(< __SumType_RefType_12550977140906565614_2 as
    __Sumtype_TypeRef_Trait_14187255806582459617 < K > > :: Type),
    __Uninhabited((:: core :: convert :: Infallible, :: core :: marker ::
    PhantomData <K >)),
} #[doc(hidden)] #[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_14187255806582459617 <K > {} impl <K,
__Sumtype_TypeParam > __Sumtype_ConstraintExprTrait_14187255806582459617 <K >
for __Sumtype_TypeParam where __Sumtype_TypeParam :
__Sumtype_ConstraintExprTrait_0_5434102876056098127 <K >, {} :: parametrized
:: _imp :: sumtype :: traits :: Iterator!
(__Sumtype_ConstraintExprTrait_0_5434102876056098127, :: parametrized :: _imp
:: sumtype :: traits :: Iterator, __Sumtype_Enum_14187255806582459617, [],
[__SumType_Variant_0 :< __SumType_RefType_3587124120095538834_0 as
__Sumtype_TypeRef_Trait_14187255806582459617 < K > > :: Type,
__SumType_Variant_1 :< __SumType_RefType_18391022986078541733_1 as
__Sumtype_TypeRef_Trait_14187255806582459617 < K > > :: Type,
__SumType_Variant_2 :< __SumType_RefType_12550977140906565614_2 as
__Sumtype_TypeRef_Trait_14187255806582459617 < K > > :: Type], [K], [K], {},);
#[allow(non_local_definitions)] impl < K > :: parametrized ::
ParametrizedIntoIter < 0usize > for Enum1 < K >
{
    type IntoIter = __Sumtype_Enum_14187255806582459617 < K > ; fn
    param_into_iter(self) -> Self :: IntoIter
    {
        #[allow(unused)] match self
        {
            Enum1 :: V1 =>
            {
                {
                    impl < K, > __Sumtype_TypeRef_Trait_14187255806582459617 < K
                    > for __SumType_RefType_3587124120095538834_0
                    { type Type = :: core :: iter :: Empty < K > ; } fn
                    __sum_type_id_fn_6023393233566934661 < K, __SumType_T :
                    __Sumtype_ConstraintExprTrait_14187255806582459617 < K > >
                    (t : __SumType_T) -> __SumType_T { t }
                    __sum_type_id_fn_6023393233566934661 :: < K, _ >
                    (__Sumtype_Enum_14187255806582459617 ::
                    __SumType_Variant_0(:: core :: iter :: empty()))
                }
            } Enum1 :: V2(__parametric_type_id_0) =>
            {
                {
                    impl < K, > __Sumtype_TypeRef_Trait_14187255806582459617 < K
                    > for __SumType_RefType_18391022986078541733_1
                    { type Type = :: core :: iter :: Once < K > ; } fn
                    __sum_type_id_fn_900566155571567194 < K, __SumType_T :
                    __Sumtype_ConstraintExprTrait_14187255806582459617 < K > >
                    (t : __SumType_T) -> __SumType_T { t }
                    __sum_type_id_fn_900566155571567194 :: < K, _ >
                    (__Sumtype_Enum_14187255806582459617 ::
                    __SumType_Variant_1(:: core :: iter ::
                    once(__parametric_type_id_0)))
                }
            } Enum1 :: V3 { _f1, _f2 } =>
            {
                {
                    impl < K, > __Sumtype_TypeRef_Trait_14187255806582459617 < K
                    > for __SumType_RefType_12550977140906565614_2
                    { type Type = :: core :: iter :: Once < K > ; } fn
                    __sum_type_id_fn_8163573599274485423 < K, __SumType_T :
                    __Sumtype_ConstraintExprTrait_14187255806582459617 < K > >
                    (t : __SumType_T) -> __SumType_T { t }
                    __sum_type_id_fn_8163573599274485423 :: < K, _ >
                    (__Sumtype_Enum_14187255806582459617 ::
                    __SumType_Variant_2(:: core :: iter :: once(_f2)))
                }
            }
        }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__sumtype_macro_5221274140601386781"
            && e.input == r#"__Sumtype_ConstraintExprTrait_0_5434102876056098127, :: parametrized :: _imp
          :: sumtype :: traits :: Iterator, __Sumtype_Enum_14187255806582459617, [],
          [__SumType_Variant_0 :< __SumType_RefType_3587124120095538834_0 as
          __Sumtype_TypeRef_Trait_14187255806582459617 < K > > :: Type,
          __SumType_Variant_1 :< __SumType_RefType_18391022986078541733_1 as
          __Sumtype_TypeRef_Trait_14187255806582459617 < K > > :: Type,
          __SumType_Variant_2 :< __SumType_RefType_12550977140906565614_2 as
          __Sumtype_TypeRef_Trait_14187255806582459617 < K > > :: Type], [K], [K], {},"#
            && e.to == r#"$crate :: _sumtrait_internal!
          ({
              __Sumtype_ConstraintExprTrait_0_5434102876056098127, :: parametrized ::
              _imp :: sumtype :: traits :: Iterator,
              __Sumtype_Enum_14187255806582459617, [],
              [__SumType_Variant_0 :< __SumType_RefType_3587124120095538834_0 as
              __Sumtype_TypeRef_Trait_14187255806582459617 < K > > :: Type,
              __SumType_Variant_1 :< __SumType_RefType_18391022986078541733_1 as
              __Sumtype_TypeRef_Trait_14187255806582459617 < K > > :: Type,
              __SumType_Variant_2 :< __SumType_RefType_12550977140906565614_2 as
              __Sumtype_TypeRef_Trait_14187255806582459617 < K > > :: Type], [K], [K],
              {},
          } [],
          {
              #[doc =
              " Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`]."]
              #[allow(private_bounds)] pub trait Iterator
              {
                  type Item; fn next(& mut self) -> :: core :: option :: Option < Self
                  :: Item > ;
              }
          }, 15686212630352170898usize, $crate, $crate :: traits :: Marker,
          [:: core :: iter :: Iterator], [], [],);"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "_sumtrait_internal"
            && e.input == r#"{
    __Sumtype_ConstraintExprTrait_0_5434102876056098127, :: parametrized ::
    _imp :: sumtype :: traits :: Iterator,
    __Sumtype_Enum_14187255806582459617, [],
    [__SumType_Variant_0 :< __SumType_RefType_3587124120095538834_0 as
    __Sumtype_TypeRef_Trait_14187255806582459617 < K > > :: Type,
    __SumType_Variant_1 :< __SumType_RefType_18391022986078541733_1 as
    __Sumtype_TypeRef_Trait_14187255806582459617 < K > > :: Type,
    __SumType_Variant_2 :< __SumType_RefType_12550977140906565614_2 as
    __Sumtype_TypeRef_Trait_14187255806582459617 < K > > :: Type], [K], [K],
    {},
} [],
{
    #[doc =
    " Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`]."]
    #[allow(private_bounds)] pub trait Iterator
    {
        type Item; fn next(& mut self) -> :: core :: option :: Option < Self
        :: Item > ;
    }
}, 15686212630352170898usize, $crate, $crate :: traits :: Marker,
[:: core :: iter :: Iterator], [], [],"#
            && e.to == r#"#[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_0_5434102876056098127 <K > {} impl <K,
__SumType_AssocType_Item > __Sumtype_ConstraintExprTrait_0_5434102876056098127
<K > for __Sumtype_Enum_14187255806582459617 <K > where <
__SumType_RefType_3587124120095538834_0 as
__Sumtype_TypeRef_Trait_14187255806582459617 < K > > :: Type : :: core :: iter
:: Iterator < Item = __SumType_AssocType_Item >, <
__SumType_RefType_18391022986078541733_1 as
__Sumtype_TypeRef_Trait_14187255806582459617 < K > > :: Type : :: core :: iter
:: Iterator < Item = __SumType_AssocType_Item >, <
__SumType_RefType_12550977140906565614_2 as
__Sumtype_TypeRef_Trait_14187255806582459617 < K > > :: Type : :: core :: iter
:: Iterator < Item = __SumType_AssocType_Item >, {} impl <K,
__SumType_AssocType_Item > :: core :: iter :: Iterator <> for
__Sumtype_Enum_14187255806582459617 <K > where <
__SumType_RefType_3587124120095538834_0 as
__Sumtype_TypeRef_Trait_14187255806582459617 < K > > :: Type : :: core :: iter
:: Iterator < Item = __SumType_AssocType_Item >, <
__SumType_RefType_18391022986078541733_1 as
__Sumtype_TypeRef_Trait_14187255806582459617 < K > > :: Type : :: core :: iter
:: Iterator < Item = __SumType_AssocType_Item >, <
__SumType_RefType_12550977140906565614_2 as
__Sumtype_TypeRef_Trait_14187255806582459617 < K > > :: Type : :: core :: iter
:: Iterator < Item = __SumType_AssocType_Item >,
{
    type Item = __SumType_AssocType_Item; fn next(& mut self) -> :: core ::
    option :: Option < Self :: Item >
    {
        match self
        {
            __Sumtype_Enum_14187255806582459617
            ::__SumType_Variant_0(__sumtrait_self_arg) => <<
            __SumType_RefType_3587124120095538834_0 as
            __Sumtype_TypeRef_Trait_14187255806582459617 < K > > :: Type as ::
            core :: iter :: Iterator < >>::next(__sumtrait_self_arg),
            __Sumtype_Enum_14187255806582459617
            ::__SumType_Variant_1(__sumtrait_self_arg) => <<
            __SumType_RefType_18391022986078541733_1 as
            __Sumtype_TypeRef_Trait_14187255806582459617 < K > > :: Type as ::
            core :: iter :: Iterator < >>::next(__sumtrait_self_arg),
            __Sumtype_Enum_14187255806582459617
            ::__SumType_Variant_2(__sumtrait_self_arg) => <<
            __SumType_RefType_12550977140906565614_2 as
            __Sumtype_TypeRef_Trait_14187255806582459617 < K > > :: Type as ::
            core :: iter :: Iterator < >>::next(__sumtrait_self_arg), Self ::
            __Uninhabited(_) => :: core :: unreachable! (),
        }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_impl_trait"
            && e.input == r#"[map, into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self = Vec<T>,
            self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }
            {
                f = f, T = M, Mapped = Vec<M>, param_map =
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() },
            }"#
            && e.to == r#"impl < T, M > ParametrizedMap < 0, M > for Vec<T>
            {
                type Mapped = Vec<M>; fn
                param_map(self, f : impl FnMut(Self :: Item) -> M) -> Self :: Mapped
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() }
            } emit_impl_trait!
            ([into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self = Vec<T>, self =
            self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            });"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "impl_all"
            && e.input == r#"[T] map, into_iter, iter_mut for Vec<T>, T = M, Mapped = Vec<M>; [T] into_iter
            for std::collections::BTreeSet<T>; [T] into_iter for
            std::collections::HashSet<T>; [T] into_iter for
            std::collections::BinaryHeap<T>; [T] map, into_iter, iter_mut for
            std::collections::LinkedList<T>, T = M, Mapped =
            std::collections::LinkedList<M>; [T] map, into_iter, iter_mut for
            std::collections::VecDeque<T>, T = M, Mapped = std::collections::VecDeque<M>;"#
            && e.to == r#"emit_impl_trait!
            ([map, into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self = Vec<T>,
            self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }
            {
                f = f, T = M, Mapped = Vec<M>, param_map =
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() },
            }); emit_impl_trait!
            ([into_iter,] impl_generics = [T], PARAM = 0, Self =
            std::collections::BTreeSet<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }); emit_impl_trait!
            ([into_iter,] impl_generics = [T], PARAM = 0, Self =
            std::collections::HashSet<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }); emit_impl_trait!
            ([into_iter,] impl_generics = [T], PARAM = 0, Self =
            std::collections::BinaryHeap<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }); emit_impl_trait!
            ([map, into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self =
            std::collections::LinkedList<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }
            {
                f = f, T = M, Mapped = std::collections::LinkedList<M>, param_map =
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() },
            }); emit_impl_trait!
            ([map, into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self =
            std::collections::VecDeque<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }
            {
                f = f, T = M, Mapped = std::collections::VecDeque<M>, param_map =
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() },
            });"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "impl_for_tuple"
            && e.input == r#"[] T []"#
            && e.to == r#"impl < T > Parametrized < {impl_for_tuple! (@ count)}> for (T,)
            {
                type Item = T; const MIN_LEN : usize = 1; const MAX_LEN : Option < usize >
                = Some(1); fn param_len(& self) -> usize { 1 } type Iter < 'a > = :: core
                :: iter :: Once < & 'a Self :: Item > where (Self, Self :: Item): 'a; fn
                param_iter < 'a > (& 'a self) -> Self :: Iter < 'a > where Self :: Item :
                'a
                {
                    :: core :: iter ::
                    once(impl_for_tuple!
                    (@ nth []
                    [& self.0, & self.1, & self.2, & self.3, & self.4, & self.5, & self.6,
                    & self.7, & self.8, & self.9, & self.10, & self.11]))
                }
            } impl < T > ParametrizedIterMut < {impl_for_tuple! (@ count)}> for (T,)
            {
                type IterMut < 'a > = :: core :: iter :: Once < & 'a mut Self :: Item >
                where (Self, Self :: Item): 'a; fn param_iter_mut < 'a > (& 'a mut self)
                -> Self :: IterMut < 'a > where Self :: Item : 'a
                {
                    core :: iter ::
                    once(impl_for_tuple!
                    (@ nth []
                    [& mut self.0, & mut self.1, & mut self.2, & mut self.3, & mut self.4,
                    & mut self.5, & mut self.6, & mut self.7, & mut self.8, & mut self.9,
                    & mut self.10, & mut self.11]))
                }
            } impl < T > ParametrizedIntoIter < {impl_for_tuple! (@ count)}> for (T,)
            {
                type IntoIter = :: core :: iter :: Once < Self :: Item > where Self ::
                Item : Sized; fn param_into_iter(self) -> Self :: IntoIter where Self ::
                Item : Sized
                {
                    core :: iter ::
                    once(impl_for_tuple!
                    (@ nth []
                    [self.0, self.1, self.2, self.3, self.4, self.5, self.6, self.7,
                    self.8, self.9, self.10, self.11]))
                }
            } impl < U, T > ParametrizedMap < {impl_for_tuple! (@ count)}, U > for (T,)
            {
                type Mapped = (U,); fn
                param_map(self, mut f : impl FnMut(Self :: Item) -> U) -> Self :: Mapped
                where Self :: Item : Sized
                {
                    impl_for_tuple!
                    (@ wrap_f f [] T []
                    [self.0, self.1, self.2, self.3, self.4, self.5, self.6, self.7,
                    self.8, self.9, self.10, self.11] {})
                }
            }"#
    }));
}

#[test]
fn external_crate_parametrized_test_tuple() {
    let expansions = run_trace_for_repo("parametrized", Some("tuple"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "parametrized"
            && e.input == r#"#[allow(unused)] struct MyStruct<T>((T, Vec<T>));"#
            && e.to == r#"#[allow(unused)] struct MyStruct < T > ((T, Vec < T >)); impl < T > ::
parametrized :: ParametrizedIntoIter <0usize > for MyStruct < T >
{
    type IntoIter = :: core :: iter :: Chain < :: core :: iter :: Once < T > ,
    :: parametrized :: Flatten < :: core :: iter :: Map < < Vec < T > as ::
    parametrized :: ParametrizedIntoIter < 0usize > > :: IntoIter, fn(T) -> ::
    core :: iter :: Once < T > > , :: core :: iter :: Once < T > > > ; fn
    param_into_iter(self) -> Self :: IntoIter
    {
        :: core :: iter ::
        once(self.0.0).chain({
            let __parametrized_fn : fn(T) -> _ = | __parametrized_arg |
            { :: core :: iter :: once(__parametrized_arg) }; :: parametrized
            :: Flatten ::
            new(< Vec < T > as :: parametrized :: ParametrizedIntoIter <
            0usize > > :: param_into_iter(self.0.1).map(__parametrized_fn))
        })
    }
} impl < T > :: parametrized :: Parametrized <0usize > for MyStruct < T >
{
    type Item = T; const MIN_LEN : usize =
    {
        const fn __parametric_type_min(a : usize, b : usize) -> usize
        { if a < b { a } else { b } }
        (< (T, Vec < T >) as :: parametrized :: Parametrized < 0usize > > ::
        MIN_LEN * 1usize + < (T, Vec < T >) as :: parametrized :: Parametrized
        < 1usize > > :: MIN_LEN * < Vec < T > as :: parametrized ::
        Parametrized < 0usize > > :: MIN_LEN * 1usize)
    }; const MAX_LEN : Option < usize > =
    {
        const fn
        __parametric_type_max(a : Option < usize > , b : Option < usize >) ->
        Option < usize >
        {
            match (a, b)
            {
                (Some(a), Some(b)) => if a > b { Some(a) } else { Some(b) } _
                => None,
            }
        } if let (Some(l), Some(r)) =
        (if let (Some(l), Some(r)) =
        (< (T, Vec < T >) as :: parametrized :: Parametrized < 0usize > > ::
        MAX_LEN, :: core :: option :: Option :: Some(1usize)) { Some(l * r) }
        else { None }, if let (Some(l), Some(r)) =
        (< (T, Vec < T >) as :: parametrized :: Parametrized < 1usize > > ::
        MAX_LEN, if let (Some(l), Some(r)) =
        (< Vec < T > as :: parametrized :: Parametrized < 0usize > > ::
        MAX_LEN, :: core :: option :: Option :: Some(1usize)) { Some(l * r) }
        else { None }) { Some(l * r) } else { None }) { Some(l + r) } else
        { None }
    }; fn param_len(& self) -> usize
    {
        (< (T, Vec < T >) as :: parametrized :: Parametrized < 0usize > > ::
        param_iter(& self.0).map(| __parametrized_arg | 1usize).sum :: < ::
        core :: primitive :: usize > () + < (T, Vec < T >) as :: parametrized
        :: Parametrized < 1usize > > ::
        param_iter(&
        self.0).map(| __parametrized_arg | < Vec < T > as :: parametrized ::
        Parametrized < 0usize > > ::
        param_iter(__parametrized_arg).map(| __parametrized_arg | 1usize).sum
        :: < :: core :: primitive :: usize > ()).sum :: < :: core :: primitive
        :: usize > ())
    } type Iter <'__parametrized_lt > = :: core :: iter :: Chain < ::
    parametrized :: Flatten < :: core :: iter :: Map < < (T, Vec < T >) as ::
    parametrized :: Parametrized < 0usize > > :: Iter < '__parametrized_lt > ,
    fn(& '__parametrized_lt T) -> :: core :: iter :: Once < &
    '__parametrized_lt T > > , :: core :: iter :: Once < & '__parametrized_lt
    T > > , :: parametrized :: Flatten < :: core :: iter :: Map < <
    (T, Vec < T >) as :: parametrized :: Parametrized < 1usize > > :: Iter <
    '__parametrized_lt > , fn(& '__parametrized_lt Vec < T >) -> ::
    parametrized :: Flatten < :: core :: iter :: Map < < Vec < T > as ::
    parametrized :: Parametrized < 0usize > > :: Iter < '__parametrized_lt > ,
    fn(& '__parametrized_lt T) -> :: core :: iter :: Once < &
    '__parametrized_lt T > > , :: core :: iter :: Once < & '__parametrized_lt
    T > > > , :: parametrized :: Flatten < :: core :: iter :: Map < < Vec < T
    > as :: parametrized :: Parametrized < 0usize > > :: Iter <
    '__parametrized_lt > , fn(& '__parametrized_lt T) -> :: core :: iter ::
    Once < & '__parametrized_lt T > > , :: core :: iter :: Once < &
    '__parametrized_lt T > > > > where (Self, Self :: Item) :
    '__parametrized_lt; fn param_iter < '__parametrized_lt >
    (& '__parametrized_lt self) -> Self :: Iter < '__parametrized_lt > where
    Self :: Item : '__parametrized_lt
    {
        {
            let __parametrized_fn : fn(& '__parametrized_lt T) -> _ = |
            __parametrized_arg |
            { :: core :: iter :: once(__parametrized_arg) }; :: parametrized
            :: Flatten ::
            new(< (T, Vec < T >) as :: parametrized :: Parametrized < 0usize >
            > :: param_iter(& self.0).map(__parametrized_fn))
        }.chain({
            let __parametrized_fn : fn(& '__parametrized_lt Vec < T >) -> _ =
            | __parametrized_arg |
            {
                {
                    let __parametrized_fn : fn(& '__parametrized_lt T) -> _ = |
                    __parametrized_arg |
                    { :: core :: iter :: once(__parametrized_arg) }; ::
                    parametrized :: Flatten ::
                    new(< Vec < T > as :: parametrized :: Parametrized < 0usize
                    > > ::
                    param_iter(__parametrized_arg).map(__parametrized_fn))
                }
            }; :: parametrized :: Flatten ::
            new(< (T, Vec < T >) as :: parametrized :: Parametrized < 1usize >
            > :: param_iter(& self.0).map(__parametrized_fn))
        })
    }
} impl < T > :: parametrized :: ParametrizedIterMut <0usize > for MyStruct < T
>
{
    type IterMut <'__parametrized_lt > = :: core :: iter :: Chain < :: core ::
    iter :: Once < & '__parametrized_lt mut T > , :: parametrized :: Flatten <
    :: core :: iter :: Map < < Vec < T > as :: parametrized ::
    ParametrizedIterMut < 0usize > > :: IterMut < '__parametrized_lt > ,
    fn(& '__parametrized_lt mut T) -> :: core :: iter :: Once < &
    '__parametrized_lt mut T > > , :: core :: iter :: Once < &
    '__parametrized_lt mut T > > > where (Self, Self :: Item) :
    '__parametrized_lt; fn param_iter_mut < '__parametrized_lt >
    (& '__parametrized_lt mut self) -> Self :: IterMut < '__parametrized_lt >
    where Self :: Item : '__parametrized_lt
    {
        :: core :: iter ::
        once(& mut
        self.0.0).chain({
            let __parametrized_fn : fn(& '__parametrized_lt mut T) -> _ = |
            __parametrized_arg |
            { :: core :: iter :: once(__parametrized_arg) }; :: parametrized
            :: Flatten ::
            new(< Vec < T > as :: parametrized :: ParametrizedIterMut < 0usize
            > > :: param_iter_mut(& mut self.0.1).map(__parametrized_fn))
        })
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && e.input == r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#
            && e.to == r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_11177185773735460263
{
    ($ ($t : tt) *) =>
    {
        $crate :: _sumtrait_internal!
        ({ $ ($t) * } [],
        {
            #[doc =
            " Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
            #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8])
                -> :: std :: io :: Result < :: core :: primitive :: usize > ;
            }
        }, 10791577178322682870usize, $crate, $crate :: traits :: Marker,
        [:: std :: io :: Read], [], [],);
    };
} #[doc(hidden)] pub use __sumtype_macro_11177185773735460263 as Read;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_impl_trait"
            && e.input == r#"[map, into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self = Vec<T>,
            self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }
            {
                f = f, T = M, Mapped = Vec<M>, param_map =
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() },
            }"#
            && e.to == r#"impl < T, M > ParametrizedMap < 0, M > for Vec<T>
            {
                type Mapped = Vec<M>; fn
                param_map(self, f : impl FnMut(Self :: Item) -> M) -> Self :: Mapped
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() }
            } emit_impl_trait!
            ([into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self = Vec<T>, self =
            self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            });"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "impl_all"
            && e.input == r#"[T] map, into_iter, iter_mut for Vec<T>, T = M, Mapped = Vec<M>; [T] into_iter
            for std::collections::BTreeSet<T>; [T] into_iter for
            std::collections::HashSet<T>; [T] into_iter for
            std::collections::BinaryHeap<T>; [T] map, into_iter, iter_mut for
            std::collections::LinkedList<T>, T = M, Mapped =
            std::collections::LinkedList<M>; [T] map, into_iter, iter_mut for
            std::collections::VecDeque<T>, T = M, Mapped = std::collections::VecDeque<M>;"#
            && e.to == r#"emit_impl_trait!
            ([map, into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self = Vec<T>,
            self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }
            {
                f = f, T = M, Mapped = Vec<M>, param_map =
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() },
            }); emit_impl_trait!
            ([into_iter,] impl_generics = [T], PARAM = 0, Self =
            std::collections::BTreeSet<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }); emit_impl_trait!
            ([into_iter,] impl_generics = [T], PARAM = 0, Self =
            std::collections::HashSet<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }); emit_impl_trait!
            ([into_iter,] impl_generics = [T], PARAM = 0, Self =
            std::collections::BinaryHeap<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }); emit_impl_trait!
            ([map, into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self =
            std::collections::LinkedList<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }
            {
                f = f, T = M, Mapped = std::collections::LinkedList<M>, param_map =
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() },
            }); emit_impl_trait!
            ([map, into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self =
            std::collections::VecDeque<T>, self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {
                IntoIter = < Self as IntoIterator > :: IntoIter, param_into_iter =
                { < Self as IntoIterator > :: into_iter(self) },
            }
            {
                f = f, T = M, Mapped = std::collections::VecDeque<M>, param_map =
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() },
            });"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "impl_for_tuple"
            && e.input == r#"[] T []"#
            && e.to == r#"impl < T > Parametrized < {impl_for_tuple! (@ count)}> for (T,)
            {
                type Item = T; const MIN_LEN : usize = 1; const MAX_LEN : Option < usize >
                = Some(1); fn param_len(& self) -> usize { 1 } type Iter < 'a > = :: core
                :: iter :: Once < & 'a Self :: Item > where (Self, Self :: Item): 'a; fn
                param_iter < 'a > (& 'a self) -> Self :: Iter < 'a > where Self :: Item :
                'a
                {
                    :: core :: iter ::
                    once(impl_for_tuple!
                    (@ nth []
                    [& self.0, & self.1, & self.2, & self.3, & self.4, & self.5, & self.6,
                    & self.7, & self.8, & self.9, & self.10, & self.11]))
                }
            } impl < T > ParametrizedIterMut < {impl_for_tuple! (@ count)}> for (T,)
            {
                type IterMut < 'a > = :: core :: iter :: Once < & 'a mut Self :: Item >
                where (Self, Self :: Item): 'a; fn param_iter_mut < 'a > (& 'a mut self)
                -> Self :: IterMut < 'a > where Self :: Item : 'a
                {
                    core :: iter ::
                    once(impl_for_tuple!
                    (@ nth []
                    [& mut self.0, & mut self.1, & mut self.2, & mut self.3, & mut self.4,
                    & mut self.5, & mut self.6, & mut self.7, & mut self.8, & mut self.9,
                    & mut self.10, & mut self.11]))
                }
            } impl < T > ParametrizedIntoIter < {impl_for_tuple! (@ count)}> for (T,)
            {
                type IntoIter = :: core :: iter :: Once < Self :: Item > where Self ::
                Item : Sized; fn param_into_iter(self) -> Self :: IntoIter where Self ::
                Item : Sized
                {
                    core :: iter ::
                    once(impl_for_tuple!
                    (@ nth []
                    [self.0, self.1, self.2, self.3, self.4, self.5, self.6, self.7,
                    self.8, self.9, self.10, self.11]))
                }
            } impl < U, T > ParametrizedMap < {impl_for_tuple! (@ count)}, U > for (T,)
            {
                type Mapped = (U,); fn
                param_map(self, mut f : impl FnMut(Self :: Item) -> U) -> Self :: Mapped
                where Self :: Item : Sized
                {
                    impl_for_tuple!
                    (@ wrap_f f [] T []
                    [self.0, self.1, self.2, self.3, self.4, self.5, self.6, self.7,
                    self.8, self.9, self.10, self.11] {})
                }
            }"#
    }));
}

#[test]
fn external_crate_sumtype_show_expansion() {
    let expansions = run_trace_for_repo("sumtype", None);
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && e.input == r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#
            && e.to == r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_13105404314811431591
{
    ($ ($t : tt) *) =>
    {
        $crate :: _sumtrait_internal!
        ({ $ ($t) * } [],
        {
            #[doc =
            " Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
            #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8])
                -> :: std :: io :: Result < :: core :: primitive :: usize > ;
            }
        }, 10781646948910720492usize, $crate, $crate :: traits :: Marker,
        [:: std :: io :: Read], [], [],);
    };
} #[doc(hidden)] pub use __sumtype_macro_13105404314811431591 as Read;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_traits"
            && e.input == ""
            && e.to == r#"#[doc(hidden)] pub struct Marker(:: core :: convert :: Infallible);
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
            #[sumtrait(implement = :: std :: io :: Read, krate = $crate, marker = $crate
            :: traits :: Marker)] #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
                io :: Result < :: core :: primitive :: usize > ;
            } impl < T : :: std :: io :: Read > Read for T
            {
                fn read(& mut self, buf : & mut [u8]) -> std :: io :: Result < usize >
                { T :: read(self, buf) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`].
            #[sumtrait(implement = :: core :: iter :: Iterator, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Iterator
            {
                type Item; fn next(& mut self) -> :: core :: option :: Option < Self ::
                Item > ;
            } impl < T : :: core :: iter :: Iterator > Iterator for T
            {
                type Item = T :: Item; fn next(& mut self) -> Option < Self :: Item >
                { T :: next(self) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Copy`].
            #[sumtrait(implement = :: core :: marker :: Copy, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Copy : $crate
            :: traits :: Clone {} impl < T : :: core :: marker :: Copy > Copy for T {}
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Clone`].
            #[sumtrait(implement = :: core :: clone :: Clone, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Clone
            { fn clone(& self) -> Self; } impl < T : :: core :: clone :: Clone > Clone for
            T { fn clone(& self) -> Self { T :: clone(self) } }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Display`].
            #[sumtrait(implement = :: core :: fmt :: Display, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Display
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result;
            } impl < T : :: core :: fmt :: Display > Display for T
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result { T :: fmt(self, f) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Debug`].
            #[sumtrait(implement = :: core :: fmt :: Debug, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Debug
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result;
            } impl < T : :: core :: fmt :: Debug > Debug for T
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result { T :: fmt(self, f) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::error::Error`].
            #[sumtrait(implement = :: std :: error :: Error, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Error : $crate
            :: traits :: Debug + $crate :: traits :: Display
            {
                fn source(& self) -> :: core :: option :: Option < &
                (dyn :: std :: error :: Error + 'static)> ;
            } impl < T : :: std :: error :: Error > Error for T
            {
                fn source(& self) -> :: core :: option :: Option < &
                (dyn :: std :: error :: Error + 'static)> { T :: source(self) }
            }"#
    }));
}

#[test]
fn external_crate_sumtype_test_bug() {
    let expansions = run_trace_for_repo("sumtype", Some("bug"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && e.input == r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#
            && e.to == r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_13105404314811431591
{
    ($ ($t : tt) *) =>
    {
        $crate :: _sumtrait_internal!
        ({ $ ($t) * } [],
        {
            #[doc =
            " Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
            #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8])
                -> :: std :: io :: Result < :: core :: primitive :: usize > ;
            }
        }, 10781646948910720492usize, $crate, $crate :: traits :: Marker,
        [:: std :: io :: Read], [], [],);
    };
} #[doc(hidden)] pub use __sumtype_macro_13105404314811431591 as Read;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtype"
            && e.input == r#"impl<T> Parametrized<0usize> for E<T>
{
    type Item = T; type Iter <'__parametrized_lt > = sumtype!
    ['__parametrized_lt] where (Self, Self :: Item) : '__parametrized_lt; fn
    param_iter<'__parametrized_lt>(&'__parametrized_lt self) ->
    Self::Iter<'__parametrized_lt> where Self::Item: '__parametrized_lt,
    {
        #[allow(unused)] match self
        {
            E::E0(__parametric_type_id_0) =>
            {
                sumtype!
                ({
                    let __parametrized_fn : fn(& '__parametrized_lt T) -> _ = |
                    __parametrized_arg |
                    { :: core :: iter :: once(__parametrized_arg) }; Flatten ::
                    new(< S < T > as Parametrized < 0usize > > ::
                    param_iter(__parametric_type_id_0).map(__parametrized_fn))
                }, for <'__parametrized_lt > Flatten < :: core :: iter :: Map
                < < S < T > as Parametrized < 0usize > > :: Iter <
                '__parametrized_lt > , fn(& '__parametrized_lt T) -> :: core
                :: iter :: Once < & '__parametrized_lt T > > , :: core :: iter
                :: Once < & '__parametrized_lt T > > where T :
                '__parametrized_lt, S < T > : Parametrized < 0usize >)
            } E::E1 =>
            {
                sumtype!
                (:: core :: iter :: empty(), for <'__parametrized_lt > :: core
                :: iter :: Empty < & '__parametrized_lt T > where T :
                '__parametrized_lt, S < T > : Parametrized < 0usize >)
            }
        }
    }
}"#
            && e.to == r#"#[doc(hidden)] #[allow(non_camel_case_types)] #[allow(non_camel_case_types)]
struct __SumType_RefType_17018617975628121186_0; #[doc(hidden)]
#[allow(non_camel_case_types)] #[allow(non_camel_case_types)] struct
__SumType_RefType_10910644691997619890_1; #[doc(hidden)]
#[allow(non_camel_case_types)] trait
__Sumtype_TypeRef_Trait_9085940579754395945 <'__parametrized_lt, T :
'__parametrized_lt > { type Type; } #[doc(hidden)]
#[allow(non_camel_case_types)] pub enum __Sumtype_Enum_9085940579754395945 <
'__parametrized_lt, T : '__parametrized_lt >
{
    __SumType_Variant_0(< __SumType_RefType_17018617975628121186_0 as
    __Sumtype_TypeRef_Trait_9085940579754395945 < '__parametrized_lt, T > > ::
    Type),
    __SumType_Variant_1(< __SumType_RefType_10910644691997619890_1 as
    __Sumtype_TypeRef_Trait_9085940579754395945 < '__parametrized_lt, T > > ::
    Type),
    __Uninhabited((:: core :: convert :: Infallible, :: core :: marker ::
    PhantomData <& '__parametrized_lt () > , :: core :: marker :: PhantomData
    <T >)),
} #[doc(hidden)] #[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_9085940579754395945 <'__parametrized_lt, T :
'__parametrized_lt > {} impl <'__parametrized_lt, T : '__parametrized_lt,
__Sumtype_TypeParam > __Sumtype_ConstraintExprTrait_9085940579754395945
<'__parametrized_lt, T > for __Sumtype_TypeParam where __Sumtype_TypeParam :
__Sumtype_ConstraintExprTrait_0_8499794266931982820 <'__parametrized_lt, T >,
T : '__parametrized_lt, S < T > : Parametrized < 0usize > , {} sumtype ::
traits :: Iterator!
(__Sumtype_ConstraintExprTrait_0_8499794266931982820, sumtype :: traits ::
Iterator, __Sumtype_Enum_9085940579754395945, [],
[__SumType_Variant_0 :< __SumType_RefType_17018617975628121186_0 as
__Sumtype_TypeRef_Trait_9085940579754395945 < '__parametrized_lt, T > > ::
Type, __SumType_Variant_1 :< __SumType_RefType_10910644691997619890_1 as
__Sumtype_TypeRef_Trait_9085940579754395945 < '__parametrized_lt, T > > ::
Type], ['__parametrized_lt, T : '__parametrized_lt], ['__parametrized_lt, T],
{ T : '__parametrized_lt, S < T > : Parametrized < 0usize > },);
#[allow(non_local_definitions)] impl < T > Parametrized < 0usize > for E < T >
{
    type Item = T; type Iter < '__parametrized_lt > =
    __Sumtype_Enum_9085940579754395945 < '__parametrized_lt, T > where
    (Self, Self :: Item) : '__parametrized_lt; fn param_iter <
    '__parametrized_lt > (& '__parametrized_lt self) -> Self :: Iter <
    '__parametrized_lt > where Self :: Item : '__parametrized_lt,
    {
        #[allow(unused)] match self
        {
            E :: E0(__parametric_type_id_0) =>
            {
                {
                    impl < '__parametrized_lt, T, >
                    __Sumtype_TypeRef_Trait_9085940579754395945 <
                    '__parametrized_lt, T > for
                    __SumType_RefType_17018617975628121186_0 where T :
                    '__parametrized_lt, S < T > : Parametrized < 0usize > ,
                    {
                        type Type = Flatten < :: core :: iter :: Map < < S < T > as
                        Parametrized < 0usize > > :: Iter < '__parametrized_lt > ,
                        fn(& '__parametrized_lt T) -> :: core :: iter :: Once < &
                        '__parametrized_lt T > > , :: core :: iter :: Once < &
                        '__parametrized_lt T > > ;
                    } fn __sum_type_id_fn_7739528229992779160 <
                    '__parametrized_lt, T, __SumType_T :
                    __Sumtype_ConstraintExprTrait_9085940579754395945 <
                    '__parametrized_lt, T > > (t : __SumType_T) -> __SumType_T
                    where T : '__parametrized_lt, S < T > : Parametrized <
                    0usize > , { t } __sum_type_id_fn_7739528229992779160 :: <
                    '__parametrized_lt, T, _ >
                    (__Sumtype_Enum_9085940579754395945 ::
                    __SumType_Variant_0({
                        let __parametrized_fn : fn(& '__parametrized_lt T) -> _ = |
                        __parametrized_arg |
                        { :: core :: iter :: once(__parametrized_arg) }; Flatten ::
                        new(< S < T > as Parametrized < 0usize > > ::
                        param_iter(__parametric_type_id_0).map(__parametrized_fn))
                    }))
                }
            } E :: E1 =>
            {
                {
                    impl < '__parametrized_lt, T, >
                    __Sumtype_TypeRef_Trait_9085940579754395945 <
                    '__parametrized_lt, T > for
                    __SumType_RefType_10910644691997619890_1 where T :
                    '__parametrized_lt, S < T > : Parametrized < 0usize > ,
                    {
                        type Type = :: core :: iter :: Empty < & '__parametrized_lt
                        T > ;
                    } fn __sum_type_id_fn_17159269004395287656 <
                    '__parametrized_lt, T, __SumType_T :
                    __Sumtype_ConstraintExprTrait_9085940579754395945 <
                    '__parametrized_lt, T > > (t : __SumType_T) -> __SumType_T
                    where T : '__parametrized_lt, S < T > : Parametrized <
                    0usize > , { t } __sum_type_id_fn_17159269004395287656 :: <
                    '__parametrized_lt, T, _ >
                    (__Sumtype_Enum_9085940579754395945 ::
                    __SumType_Variant_1(:: core :: iter :: empty()))
                }
            }
        }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__sumtype_macro_1521959044408027343"
            && e.input == r#"__Sumtype_ConstraintExprTrait_0_8499794266931982820, sumtype :: traits ::
           Iterator, __Sumtype_Enum_9085940579754395945, [],
           [__SumType_Variant_0 :< __SumType_RefType_17018617975628121186_0 as
           __Sumtype_TypeRef_Trait_9085940579754395945 < '__parametrized_lt, T > > ::
           Type, __SumType_Variant_1 :< __SumType_RefType_10910644691997619890_1 as
           __Sumtype_TypeRef_Trait_9085940579754395945 < '__parametrized_lt, T > > ::
           Type], ['__parametrized_lt, T : '__parametrized_lt], ['__parametrized_lt, T],
           { T : '__parametrized_lt, S < T > : Parametrized < 0usize > },"#
            && e.to == r#"$crate :: _sumtrait_internal!
           ({
               __Sumtype_ConstraintExprTrait_0_8499794266931982820, sumtype :: traits ::
               Iterator, __Sumtype_Enum_9085940579754395945, [],
               [__SumType_Variant_0 :< __SumType_RefType_17018617975628121186_0 as
               __Sumtype_TypeRef_Trait_9085940579754395945 < '__parametrized_lt, T > > ::
               Type, __SumType_Variant_1 :< __SumType_RefType_10910644691997619890_1 as
               __Sumtype_TypeRef_Trait_9085940579754395945 < '__parametrized_lt, T > > ::
               Type], ['__parametrized_lt, T : '__parametrized_lt],
               ['__parametrized_lt, T],
               { T : '__parametrized_lt, S < T > : Parametrized < 0usize > },
           } [],
           {
               #[doc =
               " Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`]."]
               #[allow(private_bounds)] pub trait Iterator
               {
                   type Item; fn next(& mut self) -> :: core :: option :: Option < Self
                   :: Item > ;
               }
           }, 3440346043459666793usize, $crate, $crate :: traits :: Marker,
           [:: core :: iter :: Iterator], [], [],);"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "_sumtrait_internal"
            && e.input == r#"{
    __Sumtype_ConstraintExprTrait_0_8499794266931982820, sumtype :: traits ::
    Iterator, __Sumtype_Enum_9085940579754395945, [],
    [__SumType_Variant_0 :< __SumType_RefType_17018617975628121186_0 as
    __Sumtype_TypeRef_Trait_9085940579754395945 < '__parametrized_lt, T > > ::
    Type, __SumType_Variant_1 :< __SumType_RefType_10910644691997619890_1 as
    __Sumtype_TypeRef_Trait_9085940579754395945 < '__parametrized_lt, T > > ::
    Type], ['__parametrized_lt, T : '__parametrized_lt],
    ['__parametrized_lt, T],
    { T : '__parametrized_lt, S < T > : Parametrized < 0usize > },
} [],
{
    #[doc =
    " Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`]."]
    #[allow(private_bounds)] pub trait Iterator
    {
        type Item; fn next(& mut self) -> :: core :: option :: Option < Self
        :: Item > ;
    }
}, 3440346043459666793usize, $crate, $crate :: traits :: Marker,
[:: core :: iter :: Iterator], [], [],"#
            && e.to == r#"#[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_0_8499794266931982820 <'__parametrized_lt, T :
'__parametrized_lt > {} impl <'__parametrized_lt, T : '__parametrized_lt,
__SumType_AssocType_Item > __Sumtype_ConstraintExprTrait_0_8499794266931982820
<'__parametrized_lt, T > for __Sumtype_Enum_9085940579754395945
<'__parametrized_lt, T > where T : '__parametrized_lt, S < T > : Parametrized
< 0usize > , < __SumType_RefType_17018617975628121186_0 as
__Sumtype_TypeRef_Trait_9085940579754395945 < '__parametrized_lt, T > > ::
Type : :: core :: iter :: Iterator < Item = __SumType_AssocType_Item >, <
__SumType_RefType_10910644691997619890_1 as
__Sumtype_TypeRef_Trait_9085940579754395945 < '__parametrized_lt, T > > ::
Type : :: core :: iter :: Iterator < Item = __SumType_AssocType_Item >, {}
impl <'__parametrized_lt, T : '__parametrized_lt, __SumType_AssocType_Item >
:: core :: iter :: Iterator <> for __Sumtype_Enum_9085940579754395945
<'__parametrized_lt, T > where T : '__parametrized_lt, S < T > : Parametrized
< 0usize > , < __SumType_RefType_17018617975628121186_0 as
__Sumtype_TypeRef_Trait_9085940579754395945 < '__parametrized_lt, T > > ::
Type : :: core :: iter :: Iterator < Item = __SumType_AssocType_Item >, <
__SumType_RefType_10910644691997619890_1 as
__Sumtype_TypeRef_Trait_9085940579754395945 < '__parametrized_lt, T > > ::
Type : :: core :: iter :: Iterator < Item = __SumType_AssocType_Item >,
{
    type Item = __SumType_AssocType_Item; fn next(& mut self) -> :: core ::
    option :: Option < Self :: Item >
    {
        match self
        {
            __Sumtype_Enum_9085940579754395945
            ::__SumType_Variant_0(__sumtrait_self_arg) => <<
            __SumType_RefType_17018617975628121186_0 as
            __Sumtype_TypeRef_Trait_9085940579754395945 < '__parametrized_lt,
            T > > :: Type as :: core :: iter :: Iterator <
            >>::next(__sumtrait_self_arg), __Sumtype_Enum_9085940579754395945
            ::__SumType_Variant_1(__sumtrait_self_arg) => <<
            __SumType_RefType_10910644691997619890_1 as
            __Sumtype_TypeRef_Trait_9085940579754395945 < '__parametrized_lt,
            T > > :: Type as :: core :: iter :: Iterator <
            >>::next(__sumtrait_self_arg), Self :: __Uninhabited(_) => :: core
            :: unreachable! (),
        }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_traits"
            && e.input == ""
            && e.to == r#"#[doc(hidden)] pub struct Marker(:: core :: convert :: Infallible);
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
            #[sumtrait(implement = :: std :: io :: Read, krate = $crate, marker = $crate
            :: traits :: Marker)] #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
                io :: Result < :: core :: primitive :: usize > ;
            } impl < T : :: std :: io :: Read > Read for T
            {
                fn read(& mut self, buf : & mut [u8]) -> std :: io :: Result < usize >
                { T :: read(self, buf) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`].
            #[sumtrait(implement = :: core :: iter :: Iterator, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Iterator
            {
                type Item; fn next(& mut self) -> :: core :: option :: Option < Self ::
                Item > ;
            } impl < T : :: core :: iter :: Iterator > Iterator for T
            {
                type Item = T :: Item; fn next(& mut self) -> Option < Self :: Item >
                { T :: next(self) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Copy`].
            #[sumtrait(implement = :: core :: marker :: Copy, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Copy : $crate
            :: traits :: Clone {} impl < T : :: core :: marker :: Copy > Copy for T {}
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Clone`].
            #[sumtrait(implement = :: core :: clone :: Clone, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Clone
            { fn clone(& self) -> Self; } impl < T : :: core :: clone :: Clone > Clone for
            T { fn clone(& self) -> Self { T :: clone(self) } }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Display`].
            #[sumtrait(implement = :: core :: fmt :: Display, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Display
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result;
            } impl < T : :: core :: fmt :: Display > Display for T
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result { T :: fmt(self, f) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Debug`].
            #[sumtrait(implement = :: core :: fmt :: Debug, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Debug
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result;
            } impl < T : :: core :: fmt :: Debug > Debug for T
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result { T :: fmt(self, f) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::error::Error`].
            #[sumtrait(implement = :: std :: error :: Error, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Error : $crate
            :: traits :: Debug + $crate :: traits :: Display
            {
                fn source(& self) -> :: core :: option :: Option < &
                (dyn :: std :: error :: Error + 'static)> ;
            } impl < T : :: std :: error :: Error > Error for T
            {
                fn source(& self) -> :: core :: option :: Option < &
                (dyn :: std :: error :: Error + 'static)> { T :: source(self) }
            }"#
    }));
}

#[test]
fn external_crate_sumtype_test_debug_display_test() {
    let expansions = run_trace_for_repo("sumtype", Some("debug_display_test"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && e.input == r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#
            && e.to == r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_13105404314811431591
{
    ($ ($t : tt) *) =>
    {
        $crate :: _sumtrait_internal!
        ({ $ ($t) * } [],
        {
            #[doc =
            " Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
            #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8])
                -> :: std :: io :: Result < :: core :: primitive :: usize > ;
            }
        }, 10781646948910720492usize, $crate, $crate :: traits :: Marker,
        [:: std :: io :: Read], [], [],);
    };
} #[doc(hidden)] pub use __sumtype_macro_13105404314811431591 as Read;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtype"
            && e.input == r#"fn get_debug(use_a: bool) -> impl std::fmt::Debug
{
    if use_a { sumtype!(TestStructA(42)) } else
    { sumtype!(TestStructB("hello".to_string())) }
}"#
            && e.to == r#"#[doc(hidden)] #[allow(non_camel_case_types)] trait
__Sumtype_TypeRef_Trait_5456377256056299214 <> { type Type; } #[doc(hidden)]
#[allow(non_camel_case_types)] enum __Sumtype_Enum_5456377256056299214 <
__Sumtype_TypeParam_0, __Sumtype_TypeParam_1 >
{
    __SumType_Variant_0(__Sumtype_TypeParam_0),
    __SumType_Variant_1(__Sumtype_TypeParam_1),
    __Uninhabited((:: core :: convert :: Infallible,)),
} #[doc(hidden)] #[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_5456377256056299214 <> {} impl
<__Sumtype_TypeParam > __Sumtype_ConstraintExprTrait_5456377256056299214 <>
for __Sumtype_TypeParam where __Sumtype_TypeParam :
__Sumtype_ConstraintExprTrait_0_10584988322801102184 <>, {} sumtype :: traits
:: Debug!
(__Sumtype_ConstraintExprTrait_0_10584988322801102184, sumtype :: traits ::
Debug, __Sumtype_Enum_5456377256056299214,
[__Sumtype_TypeParam_0, __Sumtype_TypeParam_1],
[__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
:__Sumtype_TypeParam_1], [], [], {},); #[allow(non_local_definitions)] fn
get_debug(use_a : bool) -> impl std :: fmt :: Debug
{
    if use_a
    {
        {
            fn __sum_type_id_fn_11990715579646061794 < __SumType_T :
            __Sumtype_ConstraintExprTrait_5456377256056299214 < > >
            (t : __SumType_T) -> __SumType_T { t }
            __sum_type_id_fn_11990715579646061794 :: < _ >
            (__Sumtype_Enum_5456377256056299214 ::
            __SumType_Variant_0(TestStructA(42)))
        }
    } else
    {
        {
            fn __sum_type_id_fn_13253486500515720339 < __SumType_T :
            __Sumtype_ConstraintExprTrait_5456377256056299214 < > >
            (t : __SumType_T) -> __SumType_T { t }
            __sum_type_id_fn_13253486500515720339 :: < _ >
            (__Sumtype_Enum_5456377256056299214 ::
            __SumType_Variant_1(TestStructB("hello".to_string())))
        }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__sumtype_macro_16785459889773847049"
            && e.input == r#"__Sumtype_ConstraintExprTrait_0_10584988322801102184, sumtype :: traits ::
           Debug, __Sumtype_Enum_5456377256056299214,
           [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1],
           [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
           :__Sumtype_TypeParam_1], [], [], {},"#
            && e.to == r#"$crate :: _sumtrait_internal!
           ({
               __Sumtype_ConstraintExprTrait_0_10584988322801102184, sumtype :: traits ::
               Debug, __Sumtype_Enum_5456377256056299214,
               [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1],
               [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
               :__Sumtype_TypeParam_1], [], [], {},
           } [],
           {
               #[doc =
               " Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Debug`]."]
               #[allow(private_bounds)] pub trait Debug
               {
                   fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> ::
                   core :: fmt :: Result;
               }
           }, 350393575320764647usize, $crate, $crate :: traits :: Marker,
           [:: core :: fmt :: Debug], [], [],);"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__sumtype_macro_9237415964743334770"
            && e.input == r#"__Sumtype_ConstraintExprTrait_0_5587116923898155743, sumtype :: traits ::
           Display, __Sumtype_Enum_3871496345244515384,
           [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1],
           [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
           :__Sumtype_TypeParam_1], [], [], {},"#
            && e.to == r#"$crate :: _sumtrait_internal!
           ({
               __Sumtype_ConstraintExprTrait_0_5587116923898155743, sumtype :: traits ::
               Display, __Sumtype_Enum_3871496345244515384,
               [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1],
               [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
               :__Sumtype_TypeParam_1], [], [], {},
           } [],
           {
               #[doc =
               " Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Display`]."]
               #[allow(private_bounds)] pub trait Display
               {
                   fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> ::
                   core :: fmt :: Result;
               }
           }, 2760776475082878101usize, $crate, $crate :: traits :: Marker,
           [:: core :: fmt :: Display], [], [],);"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "_sumtrait_internal"
            && e.input == r#"{
    __Sumtype_ConstraintExprTrait_0_10584988322801102184, sumtype :: traits ::
    Debug, __Sumtype_Enum_5456377256056299214,
    [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1],
    [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
    :__Sumtype_TypeParam_1], [], [], {},
} [],
{
    #[doc =
    " Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Debug`]."]
    #[allow(private_bounds)] pub trait Debug
    {
        fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> ::
        core :: fmt :: Result;
    }
}, 350393575320764647usize, $crate, $crate :: traits :: Marker,
[:: core :: fmt :: Debug], [], [],"#
            && e.to == r#"#[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_0_10584988322801102184 <> {} impl
<__Sumtype_TypeParam_0, __Sumtype_TypeParam_1 >
__Sumtype_ConstraintExprTrait_0_10584988322801102184 <> for
__Sumtype_Enum_5456377256056299214 <__Sumtype_TypeParam_0,
__Sumtype_TypeParam_1 > where __Sumtype_TypeParam_0 : :: core :: fmt :: Debug
< >, __Sumtype_TypeParam_1 : :: core :: fmt :: Debug < >, {} impl
<__Sumtype_TypeParam_0, __Sumtype_TypeParam_1 > :: core :: fmt :: Debug <> for
__Sumtype_Enum_5456377256056299214 <__Sumtype_TypeParam_0,
__Sumtype_TypeParam_1 > where __Sumtype_TypeParam_0 : :: core :: fmt :: Debug
< >, __Sumtype_TypeParam_1 : :: core :: fmt :: Debug < >,
{
    fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
    fmt :: Result
    {
        match self
        {
            __Sumtype_Enum_5456377256056299214
            ::__SumType_Variant_0(__sumtrait_self_arg) =>
            <__Sumtype_TypeParam_0 as :: core :: fmt :: Debug <
            >>::fmt(__sumtrait_self_arg, f),
            __Sumtype_Enum_5456377256056299214
            ::__SumType_Variant_1(__sumtrait_self_arg) =>
            <__Sumtype_TypeParam_1 as :: core :: fmt :: Debug <
            >>::fmt(__sumtrait_self_arg, f), Self :: __Uninhabited(_) => ::
            core :: unreachable! (),
        }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_traits"
            && e.input == ""
            && e.to == r#"#[doc(hidden)] pub struct Marker(:: core :: convert :: Infallible);
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
            #[sumtrait(implement = :: std :: io :: Read, krate = $crate, marker = $crate
            :: traits :: Marker)] #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
                io :: Result < :: core :: primitive :: usize > ;
            } impl < T : :: std :: io :: Read > Read for T
            {
                fn read(& mut self, buf : & mut [u8]) -> std :: io :: Result < usize >
                { T :: read(self, buf) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`].
            #[sumtrait(implement = :: core :: iter :: Iterator, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Iterator
            {
                type Item; fn next(& mut self) -> :: core :: option :: Option < Self ::
                Item > ;
            } impl < T : :: core :: iter :: Iterator > Iterator for T
            {
                type Item = T :: Item; fn next(& mut self) -> Option < Self :: Item >
                { T :: next(self) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Copy`].
            #[sumtrait(implement = :: core :: marker :: Copy, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Copy : $crate
            :: traits :: Clone {} impl < T : :: core :: marker :: Copy > Copy for T {}
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Clone`].
            #[sumtrait(implement = :: core :: clone :: Clone, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Clone
            { fn clone(& self) -> Self; } impl < T : :: core :: clone :: Clone > Clone for
            T { fn clone(& self) -> Self { T :: clone(self) } }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Display`].
            #[sumtrait(implement = :: core :: fmt :: Display, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Display
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result;
            } impl < T : :: core :: fmt :: Display > Display for T
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result { T :: fmt(self, f) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Debug`].
            #[sumtrait(implement = :: core :: fmt :: Debug, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Debug
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result;
            } impl < T : :: core :: fmt :: Debug > Debug for T
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result { T :: fmt(self, f) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::error::Error`].
            #[sumtrait(implement = :: std :: error :: Error, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Error : $crate
            :: traits :: Debug + $crate :: traits :: Display
            {
                fn source(& self) -> :: core :: option :: Option < &
                (dyn :: std :: error :: Error + 'static)> ;
            } impl < T : :: std :: error :: Error > Error for T
            {
                fn source(& self) -> :: core :: option :: Option < &
                (dyn :: std :: error :: Error + 'static)> { T :: source(self) }
            }"#
    }));
}

#[test]
fn external_crate_sumtype_test_error_test() {
    let expansions = run_trace_for_repo("sumtype", Some("error_test"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && e.input == r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#
            && e.to == r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_13105404314811431591
{
    ($ ($t : tt) *) =>
    {
        $crate :: _sumtrait_internal!
        ({ $ ($t) * } [],
        {
            #[doc =
            " Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
            #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8])
                -> :: std :: io :: Result < :: core :: primitive :: usize > ;
            }
        }, 10781646948910720492usize, $crate, $crate :: traits :: Marker,
        [:: std :: io :: Read], [], [],);
    };
} #[doc(hidden)] pub use __sumtype_macro_13105404314811431591 as Read;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtype"
            && e.input == r#"fn get_error(error_type: &str) -> impl std::error::Error
{
    match error_type
    {
        "io" => sumtype!(IoError("Failed to read file".to_string())), "parse"
        => sumtype!(ParseError("Invalid JSON format".to_string())), "network"
        =>
        sumtype!(NetworkError
        { code: 404, message: "Not Found".to_string() }), _ =>
        sumtype!(IoError("Unknown error".to_string())),
    }
}"#
            && e.to == r#"#[doc(hidden)] #[allow(non_camel_case_types)] trait
__Sumtype_TypeRef_Trait_18251582750965134174 <> { type Type; } #[doc(hidden)]
#[allow(non_camel_case_types)] enum __Sumtype_Enum_18251582750965134174 <
__Sumtype_TypeParam_0, __Sumtype_TypeParam_1, __Sumtype_TypeParam_2,
__Sumtype_TypeParam_3 >
{
    __SumType_Variant_0(__Sumtype_TypeParam_0),
    __SumType_Variant_1(__Sumtype_TypeParam_1),
    __SumType_Variant_2(__Sumtype_TypeParam_2),
    __SumType_Variant_3(__Sumtype_TypeParam_3),
    __Uninhabited((:: core :: convert :: Infallible,)),
} #[doc(hidden)] #[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_18251582750965134174 <> {} impl
<__Sumtype_TypeParam > __Sumtype_ConstraintExprTrait_18251582750965134174 <>
for __Sumtype_TypeParam where __Sumtype_TypeParam :
__Sumtype_ConstraintExprTrait_0_11652738743841793435 <>, {} sumtype :: traits
:: Error!
(__Sumtype_ConstraintExprTrait_0_11652738743841793435, sumtype :: traits ::
Error, __Sumtype_Enum_18251582750965134174,
[__Sumtype_TypeParam_0, __Sumtype_TypeParam_1, __Sumtype_TypeParam_2,
__Sumtype_TypeParam_3],
[__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
:__Sumtype_TypeParam_1, __SumType_Variant_2 :__Sumtype_TypeParam_2,
__SumType_Variant_3 :__Sumtype_TypeParam_3], [], [], {},);
#[allow(non_local_definitions)] fn get_error(error_type : & str) -> impl std
:: error :: Error
{
    match error_type
    {
        "io" =>
        {
            fn __sum_type_id_fn_9602260471403944929 < __SumType_T :
            __Sumtype_ConstraintExprTrait_18251582750965134174 < > >
            (t : __SumType_T) -> __SumType_T { t }
            __sum_type_id_fn_9602260471403944929 :: < _ >
            (__Sumtype_Enum_18251582750965134174 ::
            __SumType_Variant_0(IoError("Failed to read file".to_string())))
        }, "parse" =>
        {
            fn __sum_type_id_fn_15343232208450129727 < __SumType_T :
            __Sumtype_ConstraintExprTrait_18251582750965134174 < > >
            (t : __SumType_T) -> __SumType_T { t }
            __sum_type_id_fn_15343232208450129727 :: < _ >
            (__Sumtype_Enum_18251582750965134174 ::
            __SumType_Variant_1(ParseError("Invalid JSON format".to_string())))
        }, "network" =>
        {
            fn __sum_type_id_fn_12753680546711347366 < __SumType_T :
            __Sumtype_ConstraintExprTrait_18251582750965134174 < > >
            (t : __SumType_T) -> __SumType_T { t }
            __sum_type_id_fn_12753680546711347366 :: < _ >
            (__Sumtype_Enum_18251582750965134174 ::
            __SumType_Variant_2(NetworkError
            { code : 404, message : "Not Found".to_string() }))
        }, _ =>
        {
            fn __sum_type_id_fn_11938447556633076789 < __SumType_T :
            __Sumtype_ConstraintExprTrait_18251582750965134174 < > >
            (t : __SumType_T) -> __SumType_T { t }
            __sum_type_id_fn_11938447556633076789 :: < _ >
            (__Sumtype_Enum_18251582750965134174 ::
            __SumType_Variant_3(IoError("Unknown error".to_string())))
        },
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__sumtype_macro_18238273585450378592"
            && e.input == r#"__Sumtype_ConstraintExprTrait_0_11652738743841793435, sumtype :: traits ::
           Error, __Sumtype_Enum_18251582750965134174,
           [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1, __Sumtype_TypeParam_2,
           __Sumtype_TypeParam_3],
           [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
           :__Sumtype_TypeParam_1, __SumType_Variant_2 :__Sumtype_TypeParam_2,
           __SumType_Variant_3 :__Sumtype_TypeParam_3], [], [], {},"#
            && e.to == r#"$crate :: _sumtrait_internal!
           ({
               __Sumtype_ConstraintExprTrait_0_11652738743841793435, sumtype :: traits ::
               Error, __Sumtype_Enum_18251582750965134174,
               [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1, __Sumtype_TypeParam_2,
               __Sumtype_TypeParam_3],
               [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
               :__Sumtype_TypeParam_1, __SumType_Variant_2 :__Sumtype_TypeParam_2,
               __SumType_Variant_3 :__Sumtype_TypeParam_3], [], [], {},
           } [],
           {
               #[doc =
               " Target of [`sumtype::sumtype`] macro, which implements [`std::error::Error`]."]
               #[allow(private_bounds)] pub trait Error : $crate :: traits :: Debug +
               $crate :: traits :: Display
               {
                   fn source(& self) -> :: core :: option :: Option < &
                   (dyn :: std :: error :: Error + 'static) > ;
               }
           }, 12401079342059944432usize, $crate, $crate :: traits :: Marker,
           [:: std :: error :: Error],
           [$crate :: traits :: Debug, $crate :: traits :: Display], [],);"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__sumtype_macro_16785459889773847049"
            && e.input == r#"__SumTrait_ConstraintTrait_0_18102889824935346339, sumtype :: traits :: Error,
           __Sumtype_Enum_18251582750965134174,
           [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1, __Sumtype_TypeParam_2,
           __Sumtype_TypeParam_3],
           [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
           :__Sumtype_TypeParam_1, __SumType_Variant_2 :__Sumtype_TypeParam_2,
           __SumType_Variant_3 :__Sumtype_TypeParam_3], [], [], {},"#
            && e.to == r#"$crate :: _sumtrait_internal!
           ({
               __SumTrait_ConstraintTrait_0_18102889824935346339, sumtype :: traits ::
               Error, __Sumtype_Enum_18251582750965134174,
               [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1, __Sumtype_TypeParam_2,
               __Sumtype_TypeParam_3],
               [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
               :__Sumtype_TypeParam_1, __SumType_Variant_2 :__Sumtype_TypeParam_2,
               __SumType_Variant_3 :__Sumtype_TypeParam_3], [], [], {},
           } [],
           {
               #[doc =
               " Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Debug`]."]
               #[allow(private_bounds)] pub trait Debug
               {
                   fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> ::
                   core :: fmt :: Result;
               }
           }, 350393575320764647usize, $crate, $crate :: traits :: Marker,
           [:: core :: fmt :: Debug], [], [],);"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__sumtype_macro_9237415964743334770"
            && e.input == r#"__SumTrait_ConstraintTrait_1_13478407474903257517, sumtype :: traits :: Error,
           __Sumtype_Enum_18251582750965134174,
           [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1, __Sumtype_TypeParam_2,
           __Sumtype_TypeParam_3],
           [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
           :__Sumtype_TypeParam_1, __SumType_Variant_2 :__Sumtype_TypeParam_2,
           __SumType_Variant_3 :__Sumtype_TypeParam_3], [], [], {},"#
            && e.to == r#"$crate :: _sumtrait_internal!
           ({
               __SumTrait_ConstraintTrait_1_13478407474903257517, sumtype :: traits ::
               Error, __Sumtype_Enum_18251582750965134174,
               [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1, __Sumtype_TypeParam_2,
               __Sumtype_TypeParam_3],
               [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
               :__Sumtype_TypeParam_1, __SumType_Variant_2 :__Sumtype_TypeParam_2,
               __SumType_Variant_3 :__Sumtype_TypeParam_3], [], [], {},
           } [],
           {
               #[doc =
               " Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Display`]."]
               #[allow(private_bounds)] pub trait Display
               {
                   fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> ::
                   core :: fmt :: Result;
               }
           }, 2760776475082878101usize, $crate, $crate :: traits :: Marker,
           [:: core :: fmt :: Display], [], [],);"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "_sumtrait_internal"
            && e.input == r#"{
    __Sumtype_ConstraintExprTrait_0_11652738743841793435, sumtype :: traits ::
    Error, __Sumtype_Enum_18251582750965134174,
    [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1, __Sumtype_TypeParam_2,
    __Sumtype_TypeParam_3],
    [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
    :__Sumtype_TypeParam_1, __SumType_Variant_2 :__Sumtype_TypeParam_2,
    __SumType_Variant_3 :__Sumtype_TypeParam_3], [], [], {},
} [],
{
    #[doc =
    " Target of [`sumtype::sumtype`] macro, which implements [`std::error::Error`]."]
    #[allow(private_bounds)] pub trait Error : $crate :: traits :: Debug +
    $crate :: traits :: Display
    {
        fn source(& self) -> :: core :: option :: Option < &
        (dyn :: std :: error :: Error + 'static) > ;
    }
}, 12401079342059944432usize, $crate, $crate :: traits :: Marker,
[:: std :: error :: Error],
[$crate :: traits :: Debug, $crate :: traits :: Display], [],"#
            && e.to == r#"$crate :: traits :: Debug!
(__SumTrait_ConstraintTrait_0_18102889824935346339, sumtype :: traits ::
Error, __Sumtype_Enum_18251582750965134174,
[__Sumtype_TypeParam_0, __Sumtype_TypeParam_1, __Sumtype_TypeParam_2,
__Sumtype_TypeParam_3],
[__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
:__Sumtype_TypeParam_1, __SumType_Variant_2 :__Sumtype_TypeParam_2,
__SumType_Variant_3 :__Sumtype_TypeParam_3], [], [], {},); $crate :: traits ::
Display!
(__SumTrait_ConstraintTrait_1_13478407474903257517, sumtype :: traits ::
Error, __Sumtype_Enum_18251582750965134174,
[__Sumtype_TypeParam_0, __Sumtype_TypeParam_1, __Sumtype_TypeParam_2,
__Sumtype_TypeParam_3],
[__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
:__Sumtype_TypeParam_1, __SumType_Variant_2 :__Sumtype_TypeParam_2,
__SumType_Variant_3 :__Sumtype_TypeParam_3], [], [], {},);
#[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_0_11652738743841793435 <> {} impl
<__Sumtype_TypeParam_0, __Sumtype_TypeParam_1, __Sumtype_TypeParam_2,
__Sumtype_TypeParam_3 > __Sumtype_ConstraintExprTrait_0_11652738743841793435
<> for __Sumtype_Enum_18251582750965134174 <__Sumtype_TypeParam_0,
__Sumtype_TypeParam_1, __Sumtype_TypeParam_2, __Sumtype_TypeParam_3 > where
__Sumtype_TypeParam_0 : :: std :: error :: Error < >, __Sumtype_TypeParam_1 :
:: std :: error :: Error < >, __Sumtype_TypeParam_2 : :: std :: error :: Error
< >, __Sumtype_TypeParam_3 : :: std :: error :: Error < >, Self :
__SumTrait_ConstraintTrait_0_18102889824935346339 <>, Self :
__SumTrait_ConstraintTrait_1_13478407474903257517 <>, {} impl
<__Sumtype_TypeParam_0, __Sumtype_TypeParam_1, __Sumtype_TypeParam_2,
__Sumtype_TypeParam_3 > :: std :: error :: Error <> for
__Sumtype_Enum_18251582750965134174 <__Sumtype_TypeParam_0,
__Sumtype_TypeParam_1, __Sumtype_TypeParam_2, __Sumtype_TypeParam_3 > where
__Sumtype_TypeParam_0 : :: std :: error :: Error < >, __Sumtype_TypeParam_1 :
:: std :: error :: Error < >, __Sumtype_TypeParam_2 : :: std :: error :: Error
< >, __Sumtype_TypeParam_3 : :: std :: error :: Error < >, Self : $crate ::
traits :: Debug, Self : $crate :: traits :: Display,
{
    fn source(& self) -> :: core :: option :: Option < &
    (dyn :: std :: error :: Error + 'static) >
    {
        match self
        {
            __Sumtype_Enum_18251582750965134174
            ::__SumType_Variant_0(__sumtrait_self_arg) =>
            <__Sumtype_TypeParam_0 as :: std :: error :: Error <
            >>::source(__sumtrait_self_arg),
            __Sumtype_Enum_18251582750965134174
            ::__SumType_Variant_1(__sumtrait_self_arg) =>
            <__Sumtype_TypeParam_1 as :: std :: error :: Error <
            >>::source(__sumtrait_self_arg),
            __Sumtype_Enum_18251582750965134174
            ::__SumType_Variant_2(__sumtrait_self_arg) =>
            <__Sumtype_TypeParam_2 as :: std :: error :: Error <
            >>::source(__sumtrait_self_arg),
            __Sumtype_Enum_18251582750965134174
            ::__SumType_Variant_3(__sumtrait_self_arg) =>
            <__Sumtype_TypeParam_3 as :: std :: error :: Error <
            >>::source(__sumtrait_self_arg), Self :: __Uninhabited(_) => ::
            core :: unreachable! (),
        }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_traits"
            && e.input == ""
            && e.to == r#"#[doc(hidden)] pub struct Marker(:: core :: convert :: Infallible);
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
            #[sumtrait(implement = :: std :: io :: Read, krate = $crate, marker = $crate
            :: traits :: Marker)] #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
                io :: Result < :: core :: primitive :: usize > ;
            } impl < T : :: std :: io :: Read > Read for T
            {
                fn read(& mut self, buf : & mut [u8]) -> std :: io :: Result < usize >
                { T :: read(self, buf) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`].
            #[sumtrait(implement = :: core :: iter :: Iterator, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Iterator
            {
                type Item; fn next(& mut self) -> :: core :: option :: Option < Self ::
                Item > ;
            } impl < T : :: core :: iter :: Iterator > Iterator for T
            {
                type Item = T :: Item; fn next(& mut self) -> Option < Self :: Item >
                { T :: next(self) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Copy`].
            #[sumtrait(implement = :: core :: marker :: Copy, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Copy : $crate
            :: traits :: Clone {} impl < T : :: core :: marker :: Copy > Copy for T {}
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Clone`].
            #[sumtrait(implement = :: core :: clone :: Clone, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Clone
            { fn clone(& self) -> Self; } impl < T : :: core :: clone :: Clone > Clone for
            T { fn clone(& self) -> Self { T :: clone(self) } }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Display`].
            #[sumtrait(implement = :: core :: fmt :: Display, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Display
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result;
            } impl < T : :: core :: fmt :: Display > Display for T
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result { T :: fmt(self, f) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Debug`].
            #[sumtrait(implement = :: core :: fmt :: Debug, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Debug
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result;
            } impl < T : :: core :: fmt :: Debug > Debug for T
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result { T :: fmt(self, f) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::error::Error`].
            #[sumtrait(implement = :: std :: error :: Error, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Error : $crate
            :: traits :: Debug + $crate :: traits :: Display
            {
                fn source(& self) -> :: core :: option :: Option < &
                (dyn :: std :: error :: Error + 'static)> ;
            } impl < T : :: std :: error :: Error > Error for T
            {
                fn source(& self) -> :: core :: option :: Option < &
                (dyn :: std :: error :: Error + 'static)> { T :: source(self) }
            }"#
    }));
}

#[test]
fn external_crate_sumtype_test_large() {
    let expansions = run_trace_for_repo("sumtype", Some("large"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && e.input == r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#
            && e.to == r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_13105404314811431591
{
    ($ ($t : tt) *) =>
    {
        $crate :: _sumtrait_internal!
        ({ $ ($t) * } [],
        {
            #[doc =
            " Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
            #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8])
                -> :: std :: io :: Result < :: core :: primitive :: usize > ;
            }
        }, 10781646948910720492usize, $crate, $crate :: traits :: Marker,
        [:: std :: io :: Read], [], [],);
    };
} #[doc(hidden)] pub use __sumtype_macro_13105404314811431591 as Read;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtype"
            && e.input == r#"impl MyTrait for ()
{
    type Ty<'a, T> = sumtype!['a, T] where T: 'a; fn f<'a,
    T>(i: usize, t: &'a T) -> Self::Ty<'a, T>
    {
        if i == 0
        {
            sumtype!(std::iter::empty(), for<'a, T: 'a> std::iter::Empty<&'a
            T>)
        } else
        {
            sumtype!(std::iter::repeat(t).take(i), for<'a, T: 'a>
            std::iter::Take<std::iter::Repeat<&'a T>>)
        }
    }
}"#
            && e.to == r#"#[doc(hidden)] #[allow(non_camel_case_types)] #[allow(non_camel_case_types)]
struct __SumType_RefType_12165905018845594611_0; #[doc(hidden)]
#[allow(non_camel_case_types)] #[allow(non_camel_case_types)] struct
__SumType_RefType_2319952420744030785_1; #[doc(hidden)]
#[allow(non_camel_case_types)] trait
__Sumtype_TypeRef_Trait_10602359755763293886 <'a, T : 'a > { type Type; }
#[doc(hidden)] #[allow(non_camel_case_types)] pub enum
__Sumtype_Enum_10602359755763293886 < 'a, T : 'a >
{
    __SumType_Variant_0(< __SumType_RefType_12165905018845594611_0 as
    __Sumtype_TypeRef_Trait_10602359755763293886 < 'a, T > > :: Type),
    __SumType_Variant_1(< __SumType_RefType_2319952420744030785_1 as
    __Sumtype_TypeRef_Trait_10602359755763293886 < 'a, T > > :: Type),
    __Uninhabited((:: core :: convert :: Infallible, :: core :: marker ::
    PhantomData <& 'a () > , :: core :: marker :: PhantomData <T >)),
} #[doc(hidden)] #[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_10602359755763293886 <'a, T : 'a > {} impl <'a,
T : 'a, __Sumtype_TypeParam >
__Sumtype_ConstraintExprTrait_10602359755763293886 <'a, T > for
__Sumtype_TypeParam where __Sumtype_TypeParam :
__Sumtype_ConstraintExprTrait_0_14054148328570805687 <'a, T >, {} sumtype ::
traits :: Iterator!
(__Sumtype_ConstraintExprTrait_0_14054148328570805687, sumtype :: traits ::
Iterator, __Sumtype_Enum_10602359755763293886, [],
[__SumType_Variant_0 :< __SumType_RefType_12165905018845594611_0 as
__Sumtype_TypeRef_Trait_10602359755763293886 < 'a, T > > :: Type,
__SumType_Variant_1 :< __SumType_RefType_2319952420744030785_1 as
__Sumtype_TypeRef_Trait_10602359755763293886 < 'a, T > > :: Type],
['a, T : 'a], ['a, T], {},); #[allow(non_local_definitions)] impl MyTrait for
()
{
    type Ty < 'a, T > = __Sumtype_Enum_10602359755763293886 < 'a, T > where T
    : 'a; fn f < 'a, T > (i : usize, t : & 'a T) -> Self :: Ty < 'a, T >
    {
        if i == 0
        {
            {
                impl < 'a, T : 'a, >
                __Sumtype_TypeRef_Trait_10602359755763293886 < 'a, T > for
                __SumType_RefType_12165905018845594611_0
                { type Type = std :: iter :: Empty < & 'a T > ; } fn
                __sum_type_id_fn_3986676629773653238 < 'a, T : 'a, __SumType_T
                : __Sumtype_ConstraintExprTrait_10602359755763293886 < 'a, T >
                > (t : __SumType_T) -> __SumType_T { t }
                __sum_type_id_fn_3986676629773653238 :: < 'a, T, _ >
                (__Sumtype_Enum_10602359755763293886 ::
                __SumType_Variant_0(std :: iter :: empty()))
            }
        } else
        {
            {
                impl < 'a, T : 'a, >
                __Sumtype_TypeRef_Trait_10602359755763293886 < 'a, T > for
                __SumType_RefType_2319952420744030785_1
                {
                    type Type = std :: iter :: Take < std :: iter :: Repeat < &
                    'a T > > ;
                } fn __sum_type_id_fn_10092417021580348604 < 'a, T : 'a,
                __SumType_T :
                __Sumtype_ConstraintExprTrait_10602359755763293886 < 'a, T > >
                (t : __SumType_T) -> __SumType_T { t }
                __sum_type_id_fn_10092417021580348604 :: < 'a, T, _ >
                (__Sumtype_Enum_10602359755763293886 ::
                __SumType_Variant_1(std :: iter :: repeat(t).take(i)))
            }
        }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__sumtype_macro_1521959044408027343"
            && e.input == r#"__Sumtype_ConstraintExprTrait_0_14054148328570805687, sumtype :: traits ::
           Iterator, __Sumtype_Enum_10602359755763293886, [],
           [__SumType_Variant_0 :< __SumType_RefType_12165905018845594611_0 as
           __Sumtype_TypeRef_Trait_10602359755763293886 < 'a, T > > :: Type,
           __SumType_Variant_1 :< __SumType_RefType_2319952420744030785_1 as
           __Sumtype_TypeRef_Trait_10602359755763293886 < 'a, T > > :: Type],
           ['a, T : 'a], ['a, T], {},"#
            && e.to == r#"$crate :: _sumtrait_internal!
           ({
               __Sumtype_ConstraintExprTrait_0_14054148328570805687, sumtype :: traits ::
               Iterator, __Sumtype_Enum_10602359755763293886, [],
               [__SumType_Variant_0 :< __SumType_RefType_12165905018845594611_0 as
               __Sumtype_TypeRef_Trait_10602359755763293886 < 'a, T > > :: Type,
               __SumType_Variant_1 :< __SumType_RefType_2319952420744030785_1 as
               __Sumtype_TypeRef_Trait_10602359755763293886 < 'a, T > > :: Type],
               ['a, T : 'a], ['a, T], {},
           } [],
           {
               #[doc =
               " Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`]."]
               #[allow(private_bounds)] pub trait Iterator
               {
                   type Item; fn next(& mut self) -> :: core :: option :: Option < Self
                   :: Item > ;
               }
           }, 3440346043459666793usize, $crate, $crate :: traits :: Marker,
           [:: core :: iter :: Iterator], [], [],);"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "_sumtrait_internal"
            && e.input == r#"{
    __Sumtype_ConstraintExprTrait_0_14054148328570805687, sumtype :: traits ::
    Iterator, __Sumtype_Enum_10602359755763293886, [],
    [__SumType_Variant_0 :< __SumType_RefType_12165905018845594611_0 as
    __Sumtype_TypeRef_Trait_10602359755763293886 < 'a, T > > :: Type,
    __SumType_Variant_1 :< __SumType_RefType_2319952420744030785_1 as
    __Sumtype_TypeRef_Trait_10602359755763293886 < 'a, T > > :: Type],
    ['a, T : 'a], ['a, T], {},
} [],
{
    #[doc =
    " Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`]."]
    #[allow(private_bounds)] pub trait Iterator
    {
        type Item; fn next(& mut self) -> :: core :: option :: Option < Self
        :: Item > ;
    }
}, 3440346043459666793usize, $crate, $crate :: traits :: Marker,
[:: core :: iter :: Iterator], [], [],"#
            && e.to == r#"#[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_0_14054148328570805687 <'a, T : 'a > {} impl
<'a, T : 'a, __SumType_AssocType_Item >
__Sumtype_ConstraintExprTrait_0_14054148328570805687 <'a, T > for
__Sumtype_Enum_10602359755763293886 <'a, T > where <
__SumType_RefType_12165905018845594611_0 as
__Sumtype_TypeRef_Trait_10602359755763293886 < 'a, T > > :: Type : :: core ::
iter :: Iterator < Item = __SumType_AssocType_Item >, <
__SumType_RefType_2319952420744030785_1 as
__Sumtype_TypeRef_Trait_10602359755763293886 < 'a, T > > :: Type : :: core ::
iter :: Iterator < Item = __SumType_AssocType_Item >, {} impl <'a, T : 'a,
__SumType_AssocType_Item > :: core :: iter :: Iterator <> for
__Sumtype_Enum_10602359755763293886 <'a, T > where <
__SumType_RefType_12165905018845594611_0 as
__Sumtype_TypeRef_Trait_10602359755763293886 < 'a, T > > :: Type : :: core ::
iter :: Iterator < Item = __SumType_AssocType_Item >, <
__SumType_RefType_2319952420744030785_1 as
__Sumtype_TypeRef_Trait_10602359755763293886 < 'a, T > > :: Type : :: core ::
iter :: Iterator < Item = __SumType_AssocType_Item >,
{
    type Item = __SumType_AssocType_Item; fn next(& mut self) -> :: core ::
    option :: Option < Self :: Item >
    {
        match self
        {
            __Sumtype_Enum_10602359755763293886
            ::__SumType_Variant_0(__sumtrait_self_arg) => <<
            __SumType_RefType_12165905018845594611_0 as
            __Sumtype_TypeRef_Trait_10602359755763293886 < 'a, T > > :: Type
            as :: core :: iter :: Iterator < >>::next(__sumtrait_self_arg),
            __Sumtype_Enum_10602359755763293886
            ::__SumType_Variant_1(__sumtrait_self_arg) => <<
            __SumType_RefType_2319952420744030785_1 as
            __Sumtype_TypeRef_Trait_10602359755763293886 < 'a, T > > :: Type
            as :: core :: iter :: Iterator < >>::next(__sumtrait_self_arg),
            Self :: __Uninhabited(_) => :: core :: unreachable! (),
        }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_traits"
            && e.input == ""
            && e.to == r#"#[doc(hidden)] pub struct Marker(:: core :: convert :: Infallible);
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
            #[sumtrait(implement = :: std :: io :: Read, krate = $crate, marker = $crate
            :: traits :: Marker)] #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
                io :: Result < :: core :: primitive :: usize > ;
            } impl < T : :: std :: io :: Read > Read for T
            {
                fn read(& mut self, buf : & mut [u8]) -> std :: io :: Result < usize >
                { T :: read(self, buf) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`].
            #[sumtrait(implement = :: core :: iter :: Iterator, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Iterator
            {
                type Item; fn next(& mut self) -> :: core :: option :: Option < Self ::
                Item > ;
            } impl < T : :: core :: iter :: Iterator > Iterator for T
            {
                type Item = T :: Item; fn next(& mut self) -> Option < Self :: Item >
                { T :: next(self) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Copy`].
            #[sumtrait(implement = :: core :: marker :: Copy, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Copy : $crate
            :: traits :: Clone {} impl < T : :: core :: marker :: Copy > Copy for T {}
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Clone`].
            #[sumtrait(implement = :: core :: clone :: Clone, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Clone
            { fn clone(& self) -> Self; } impl < T : :: core :: clone :: Clone > Clone for
            T { fn clone(& self) -> Self { T :: clone(self) } }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Display`].
            #[sumtrait(implement = :: core :: fmt :: Display, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Display
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result;
            } impl < T : :: core :: fmt :: Display > Display for T
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result { T :: fmt(self, f) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Debug`].
            #[sumtrait(implement = :: core :: fmt :: Debug, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Debug
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result;
            } impl < T : :: core :: fmt :: Debug > Debug for T
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result { T :: fmt(self, f) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::error::Error`].
            #[sumtrait(implement = :: std :: error :: Error, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Error : $crate
            :: traits :: Debug + $crate :: traits :: Display
            {
                fn source(& self) -> :: core :: option :: Option < &
                (dyn :: std :: error :: Error + 'static)> ;
            } impl < T : :: std :: error :: Error > Error for T
            {
                fn source(& self) -> :: core :: option :: Option < &
                (dyn :: std :: error :: Error + 'static)> { T :: source(self) }
            }"#
    }));
}

#[test]
fn external_crate_sumtype_test_module() {
    let expansions = run_trace_for_repo("sumtype", Some("module"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && e.input == r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#
            && e.to == r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_13105404314811431591
{
    ($ ($t : tt) *) =>
    {
        $crate :: _sumtrait_internal!
        ({ $ ($t) * } [],
        {
            #[doc =
            " Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
            #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8])
                -> :: std :: io :: Result < :: core :: primitive :: usize > ;
            }
        }, 10781646948910720492usize, $crate, $crate :: traits :: Marker,
        [:: std :: io :: Read], [], [],);
    };
} #[doc(hidden)] pub use __sumtype_macro_13105404314811431591 as Read;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtype"
            && e.input == r#"mod my_module
{
    #[allow(unused)] pub struct MyStruct { iter: sumtype!(), } impl MyStruct
    {
        #[allow(unused)] pub fn new(flag: bool) -> Self
        {
            let iter = if flag { sumtype!(0..5, std::ops::Range<u32>) } else
            {
                sumtype!(vec![10, 20, 30].into_iter(),
                std::vec::IntoIter<u32>)
            }; MyStruct { iter }
        } #[allow(unused)] pub fn iterate(self)
        { for value in self.iter { println!("{}", value); } }
    }
}"#
            && e.to == r#"#[doc(hidden)] #[allow(non_camel_case_types)] #[allow(non_camel_case_types)]
struct __SumType_RefType_8572092736649099329_0; #[doc(hidden)]
#[allow(non_camel_case_types)] #[allow(non_camel_case_types)] struct
__SumType_RefType_16992133207354132934_1; #[doc(hidden)]
#[allow(non_camel_case_types)] trait
__Sumtype_TypeRef_Trait_16151744814453200158 <> { type Type; } #[doc(hidden)]
#[allow(non_camel_case_types)] pub enum __Sumtype_Enum_16151744814453200158 <
>
{
    __SumType_Variant_0(< __SumType_RefType_8572092736649099329_0 as
    __Sumtype_TypeRef_Trait_16151744814453200158 < > > :: Type),
    __SumType_Variant_1(< __SumType_RefType_16992133207354132934_1 as
    __Sumtype_TypeRef_Trait_16151744814453200158 < > > :: Type),
    __Uninhabited((:: core :: convert :: Infallible,)),
} #[doc(hidden)] #[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_16151744814453200158 <> {} impl
<__Sumtype_TypeParam > __Sumtype_ConstraintExprTrait_16151744814453200158 <>
for __Sumtype_TypeParam where __Sumtype_TypeParam :
__Sumtype_ConstraintExprTrait_0_12077476945157456577 <>, {} sumtype :: traits
:: Iterator!
(__Sumtype_ConstraintExprTrait_0_12077476945157456577, sumtype :: traits ::
Iterator, __Sumtype_Enum_16151744814453200158, [],
[__SumType_Variant_0 :< __SumType_RefType_8572092736649099329_0 as
__Sumtype_TypeRef_Trait_16151744814453200158 < > > :: Type,
__SumType_Variant_1 :< __SumType_RefType_16992133207354132934_1 as
__Sumtype_TypeRef_Trait_16151744814453200158 < > > :: Type], [], [], {},);
#[allow(non_local_definitions)] mod my_module
{
    #[allow(unused)] pub struct MyStruct
    { iter : super :: __Sumtype_Enum_16151744814453200158, } impl MyStruct
    {
        #[allow(unused)] pub fn new(flag : bool) -> Self
        {
            let iter = if flag
            {
                {
                    impl super :: __Sumtype_TypeRef_Trait_16151744814453200158 <
                    > for super :: __SumType_RefType_8572092736649099329_0
                    { type Type = std :: ops :: Range < u32 > ; } fn
                    __sum_type_id_fn_10123907618339456399 < __SumType_T : super
                    :: __Sumtype_ConstraintExprTrait_16151744814453200158 < > >
                    (t : __SumType_T) -> __SumType_T { t }
                    __sum_type_id_fn_10123907618339456399 :: < _ >
                    (super :: __Sumtype_Enum_16151744814453200158 ::
                    __SumType_Variant_0(0 .. 5))
                }
            } else
            {
                {
                    impl super :: __Sumtype_TypeRef_Trait_16151744814453200158 <
                    > for super :: __SumType_RefType_16992133207354132934_1
                    { type Type = std :: vec :: IntoIter < u32 > ; } fn
                    __sum_type_id_fn_9202006199726958995 < __SumType_T : super
                    :: __Sumtype_ConstraintExprTrait_16151744814453200158 < > >
                    (t : __SumType_T) -> __SumType_T { t }
                    __sum_type_id_fn_9202006199726958995 :: < _ >
                    (super :: __Sumtype_Enum_16151744814453200158 ::
                    __SumType_Variant_1(vec! [10, 20, 30].into_iter()))
                }
            }; MyStruct { iter }
        } #[allow(unused)] pub fn iterate(self)
        { for value in self.iter { println! ("{}", value); } }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__sumtype_macro_1521959044408027343"
            && e.input == r#"__Sumtype_ConstraintExprTrait_0_12077476945157456577, sumtype :: traits ::
          Iterator, __Sumtype_Enum_16151744814453200158, [],
          [__SumType_Variant_0 :< __SumType_RefType_8572092736649099329_0 as
          __Sumtype_TypeRef_Trait_16151744814453200158 < > > :: Type,
          __SumType_Variant_1 :< __SumType_RefType_16992133207354132934_1 as
          __Sumtype_TypeRef_Trait_16151744814453200158 < > > :: Type], [], [], {},"#
            && e.to == r#"$crate :: _sumtrait_internal!
          ({
              __Sumtype_ConstraintExprTrait_0_12077476945157456577, sumtype :: traits ::
              Iterator, __Sumtype_Enum_16151744814453200158, [],
              [__SumType_Variant_0 :< __SumType_RefType_8572092736649099329_0 as
              __Sumtype_TypeRef_Trait_16151744814453200158 < > > :: Type,
              __SumType_Variant_1 :< __SumType_RefType_16992133207354132934_1 as
              __Sumtype_TypeRef_Trait_16151744814453200158 < > > :: Type], [], [], {},
          } [],
          {
              #[doc =
              " Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`]."]
              #[allow(private_bounds)] pub trait Iterator
              {
                  type Item; fn next(& mut self) -> :: core :: option :: Option < Self
                  :: Item > ;
              }
          }, 3440346043459666793usize, $crate, $crate :: traits :: Marker,
          [:: core :: iter :: Iterator], [], [],);"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "_sumtrait_internal"
            && e.input == r#"{
    __Sumtype_ConstraintExprTrait_0_12077476945157456577, sumtype :: traits ::
    Iterator, __Sumtype_Enum_16151744814453200158, [],
    [__SumType_Variant_0 :< __SumType_RefType_8572092736649099329_0 as
    __Sumtype_TypeRef_Trait_16151744814453200158 < > > :: Type,
    __SumType_Variant_1 :< __SumType_RefType_16992133207354132934_1 as
    __Sumtype_TypeRef_Trait_16151744814453200158 < > > :: Type], [], [], {},
} [],
{
    #[doc =
    " Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`]."]
    #[allow(private_bounds)] pub trait Iterator
    {
        type Item; fn next(& mut self) -> :: core :: option :: Option < Self
        :: Item > ;
    }
}, 3440346043459666793usize, $crate, $crate :: traits :: Marker,
[:: core :: iter :: Iterator], [], [],"#
            && e.to == r#"#[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_0_12077476945157456577 <> {} impl
<__SumType_AssocType_Item >
__Sumtype_ConstraintExprTrait_0_12077476945157456577 <> for
__Sumtype_Enum_16151744814453200158 <> where <
__SumType_RefType_8572092736649099329_0 as
__Sumtype_TypeRef_Trait_16151744814453200158 < > > :: Type : :: core :: iter
:: Iterator < Item = __SumType_AssocType_Item >, <
__SumType_RefType_16992133207354132934_1 as
__Sumtype_TypeRef_Trait_16151744814453200158 < > > :: Type : :: core :: iter
:: Iterator < Item = __SumType_AssocType_Item >, {} impl
<__SumType_AssocType_Item > :: core :: iter :: Iterator <> for
__Sumtype_Enum_16151744814453200158 <> where <
__SumType_RefType_8572092736649099329_0 as
__Sumtype_TypeRef_Trait_16151744814453200158 < > > :: Type : :: core :: iter
:: Iterator < Item = __SumType_AssocType_Item >, <
__SumType_RefType_16992133207354132934_1 as
__Sumtype_TypeRef_Trait_16151744814453200158 < > > :: Type : :: core :: iter
:: Iterator < Item = __SumType_AssocType_Item >,
{
    type Item = __SumType_AssocType_Item; fn next(& mut self) -> :: core ::
    option :: Option < Self :: Item >
    {
        match self
        {
            __Sumtype_Enum_16151744814453200158
            ::__SumType_Variant_0(__sumtrait_self_arg) => <<
            __SumType_RefType_8572092736649099329_0 as
            __Sumtype_TypeRef_Trait_16151744814453200158 < > > :: Type as ::
            core :: iter :: Iterator < >>::next(__sumtrait_self_arg),
            __Sumtype_Enum_16151744814453200158
            ::__SumType_Variant_1(__sumtrait_self_arg) => <<
            __SumType_RefType_16992133207354132934_1 as
            __Sumtype_TypeRef_Trait_16151744814453200158 < > > :: Type as ::
            core :: iter :: Iterator < >>::next(__sumtrait_self_arg), Self ::
            __Uninhabited(_) => :: core :: unreachable! (),
        }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_traits"
            && e.input == ""
            && e.to == r#"#[doc(hidden)] pub struct Marker(:: core :: convert :: Infallible);
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
            #[sumtrait(implement = :: std :: io :: Read, krate = $crate, marker = $crate
            :: traits :: Marker)] #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
                io :: Result < :: core :: primitive :: usize > ;
            } impl < T : :: std :: io :: Read > Read for T
            {
                fn read(& mut self, buf : & mut [u8]) -> std :: io :: Result < usize >
                { T :: read(self, buf) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`].
            #[sumtrait(implement = :: core :: iter :: Iterator, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Iterator
            {
                type Item; fn next(& mut self) -> :: core :: option :: Option < Self ::
                Item > ;
            } impl < T : :: core :: iter :: Iterator > Iterator for T
            {
                type Item = T :: Item; fn next(& mut self) -> Option < Self :: Item >
                { T :: next(self) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Copy`].
            #[sumtrait(implement = :: core :: marker :: Copy, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Copy : $crate
            :: traits :: Clone {} impl < T : :: core :: marker :: Copy > Copy for T {}
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Clone`].
            #[sumtrait(implement = :: core :: clone :: Clone, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Clone
            { fn clone(& self) -> Self; } impl < T : :: core :: clone :: Clone > Clone for
            T { fn clone(& self) -> Self { T :: clone(self) } }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Display`].
            #[sumtrait(implement = :: core :: fmt :: Display, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Display
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result;
            } impl < T : :: core :: fmt :: Display > Display for T
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result { T :: fmt(self, f) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Debug`].
            #[sumtrait(implement = :: core :: fmt :: Debug, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Debug
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result;
            } impl < T : :: core :: fmt :: Debug > Debug for T
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result { T :: fmt(self, f) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::error::Error`].
            #[sumtrait(implement = :: std :: error :: Error, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Error : $crate
            :: traits :: Debug + $crate :: traits :: Display
            {
                fn source(& self) -> :: core :: option :: Option < &
                (dyn :: std :: error :: Error + 'static)> ;
            } impl < T : :: std :: error :: Error > Error for T
            {
                fn source(& self) -> :: core :: option :: Option < &
                (dyn :: std :: error :: Error + 'static)> { T :: source(self) }
            }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "println"
            && e.input == r#""{}", value"#
            && e.to == r#"{ $crate :: io :: _print($crate :: format_args_nl! ("{}", value)); }"#
    }));
}

#[test]
fn external_crate_sumtype_test_multi() {
    let expansions = run_trace_for_repo("sumtype", Some("multi"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && e.input == r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#
            && e.to == r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_13105404314811431591
{
    ($ ($t : tt) *) =>
    {
        $crate :: _sumtrait_internal!
        ({ $ ($t) * } [],
        {
            #[doc =
            " Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
            #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8])
                -> :: std :: io :: Result < :: core :: primitive :: usize > ;
            }
        }, 10781646948910720492usize, $crate, $crate :: traits :: Marker,
        [:: std :: io :: Read], [], [],);
    };
} #[doc(hidden)] pub use __sumtype_macro_13105404314811431591 as Read;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtype"
            && e.input == r#"#[allow(unused)] fn f(a: usize) -> impl Iterator<Item = usize> + Clone
{
    match a
    {
        0 => sumtype!(std::iter::empty::<usize>()), 1 =>
        sumtype!(std::iter::once(a)), _ =>
        sumtype!(std::iter::repeat(a).take(a)),
    }
}"#
            && e.to == r#"#[doc(hidden)] #[allow(non_camel_case_types)] trait
__Sumtype_TypeRef_Trait_8231541105705577287 <> { type Type; } #[doc(hidden)]
#[allow(non_camel_case_types)] enum __Sumtype_Enum_8231541105705577287 <
__Sumtype_TypeParam_0, __Sumtype_TypeParam_1, __Sumtype_TypeParam_2 >
{
    __SumType_Variant_0(__Sumtype_TypeParam_0),
    __SumType_Variant_1(__Sumtype_TypeParam_1),
    __SumType_Variant_2(__Sumtype_TypeParam_2),
    __Uninhabited((:: core :: convert :: Infallible,)),
} #[doc(hidden)] #[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_8231541105705577287 <> {} impl
<__Sumtype_TypeParam > __Sumtype_ConstraintExprTrait_8231541105705577287 <>
for __Sumtype_TypeParam where __Sumtype_TypeParam :
__Sumtype_ConstraintExprTrait_0_8333595044880151855 <>, __Sumtype_TypeParam :
__Sumtype_ConstraintExprTrait_1_11081586961500924072 <>, {} sumtype :: traits
:: Iterator!
(__Sumtype_ConstraintExprTrait_0_8333595044880151855, sumtype :: traits ::
Iterator, __Sumtype_Enum_8231541105705577287,
[__Sumtype_TypeParam_0, __Sumtype_TypeParam_1, __Sumtype_TypeParam_2],
[__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
:__Sumtype_TypeParam_1, __SumType_Variant_2 :__Sumtype_TypeParam_2], [], [],
{},); sumtype :: traits :: Clone!
(__Sumtype_ConstraintExprTrait_1_11081586961500924072, sumtype :: traits ::
Clone, __Sumtype_Enum_8231541105705577287,
[__Sumtype_TypeParam_0, __Sumtype_TypeParam_1, __Sumtype_TypeParam_2],
[__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
:__Sumtype_TypeParam_1, __SumType_Variant_2 :__Sumtype_TypeParam_2], [], [],
{},); #[allow(non_local_definitions)] #[allow(unused)] fn f(a : usize) -> impl
Iterator < Item = usize > + Clone
{
    match a
    {
        0 =>
        {
            fn __sum_type_id_fn_13255820743176427062 < __SumType_T :
            __Sumtype_ConstraintExprTrait_8231541105705577287 < > >
            (t : __SumType_T) -> __SumType_T { t }
            __sum_type_id_fn_13255820743176427062 :: < _ >
            (__Sumtype_Enum_8231541105705577287 ::
            __SumType_Variant_0(std :: iter :: empty :: < usize > ()))
        }, 1 =>
        {
            fn __sum_type_id_fn_7454766164361663248 < __SumType_T :
            __Sumtype_ConstraintExprTrait_8231541105705577287 < > >
            (t : __SumType_T) -> __SumType_T { t }
            __sum_type_id_fn_7454766164361663248 :: < _ >
            (__Sumtype_Enum_8231541105705577287 ::
            __SumType_Variant_1(std :: iter :: once(a)))
        }, _ =>
        {
            fn __sum_type_id_fn_14409565964260394798 < __SumType_T :
            __Sumtype_ConstraintExprTrait_8231541105705577287 < > >
            (t : __SumType_T) -> __SumType_T { t }
            __sum_type_id_fn_14409565964260394798 :: < _ >
            (__Sumtype_Enum_8231541105705577287 ::
            __SumType_Variant_2(std :: iter :: repeat(a).take(a)))
        },
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__sumtype_macro_1521959044408027343"
            && e.input == r#"__Sumtype_ConstraintExprTrait_0_8333595044880151855, sumtype :: traits ::
          Iterator, __Sumtype_Enum_8231541105705577287,
          [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1, __Sumtype_TypeParam_2],
          [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
          :__Sumtype_TypeParam_1, __SumType_Variant_2 :__Sumtype_TypeParam_2], [], [],
          {},"#
            && e.to == r#"$crate :: _sumtrait_internal!
          ({
              __Sumtype_ConstraintExprTrait_0_8333595044880151855, sumtype :: traits ::
              Iterator, __Sumtype_Enum_8231541105705577287,
              [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1, __Sumtype_TypeParam_2],
              [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
              :__Sumtype_TypeParam_1, __SumType_Variant_2 :__Sumtype_TypeParam_2], [],
              [], {},
          } [],
          {
              #[doc =
              " Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`]."]
              #[allow(private_bounds)] pub trait Iterator
              {
                  type Item; fn next(& mut self) -> :: core :: option :: Option < Self
                  :: Item > ;
              }
          }, 3440346043459666793usize, $crate, $crate :: traits :: Marker,
          [:: core :: iter :: Iterator], [], [],);"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__sumtype_macro_11395927097724701696"
            && e.input == r#"__Sumtype_ConstraintExprTrait_1_11081586961500924072, sumtype :: traits ::
          Clone, __Sumtype_Enum_8231541105705577287,
          [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1, __Sumtype_TypeParam_2],
          [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
          :__Sumtype_TypeParam_1, __SumType_Variant_2 :__Sumtype_TypeParam_2], [], [],
          {},"#
            && e.to == r#"$crate :: _sumtrait_internal!
          ({
              __Sumtype_ConstraintExprTrait_1_11081586961500924072, sumtype :: traits ::
              Clone, __Sumtype_Enum_8231541105705577287,
              [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1, __Sumtype_TypeParam_2],
              [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
              :__Sumtype_TypeParam_1, __SumType_Variant_2 :__Sumtype_TypeParam_2], [],
              [], {},
          } [],
          {
              #[doc =
              " Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Clone`]."]
              #[allow(private_bounds)] pub trait Clone { fn clone(& self) -> Self; }
          }, 342573295450118012usize, $crate, $crate :: traits :: Marker,
          [:: core :: clone :: Clone], [], [],);"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "_sumtrait_internal"
            && e.input == r#"{
    __Sumtype_ConstraintExprTrait_0_8333595044880151855, sumtype :: traits ::
    Iterator, __Sumtype_Enum_8231541105705577287,
    [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1, __Sumtype_TypeParam_2],
    [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
    :__Sumtype_TypeParam_1, __SumType_Variant_2 :__Sumtype_TypeParam_2], [],
    [], {},
} [],
{
    #[doc =
    " Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`]."]
    #[allow(private_bounds)] pub trait Iterator
    {
        type Item; fn next(& mut self) -> :: core :: option :: Option < Self
        :: Item > ;
    }
}, 3440346043459666793usize, $crate, $crate :: traits :: Marker,
[:: core :: iter :: Iterator], [], [],"#
            && e.to == r#"#[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_0_8333595044880151855 <> {} impl
<__SumType_AssocType_Item, __Sumtype_TypeParam_0, __Sumtype_TypeParam_1,
__Sumtype_TypeParam_2 > __Sumtype_ConstraintExprTrait_0_8333595044880151855 <>
for __Sumtype_Enum_8231541105705577287 <__Sumtype_TypeParam_0,
__Sumtype_TypeParam_1, __Sumtype_TypeParam_2 > where __Sumtype_TypeParam_0 :
:: core :: iter :: Iterator < Item = __SumType_AssocType_Item >,
__Sumtype_TypeParam_1 : :: core :: iter :: Iterator < Item =
__SumType_AssocType_Item >, __Sumtype_TypeParam_2 : :: core :: iter ::
Iterator < Item = __SumType_AssocType_Item >, {} impl
<__SumType_AssocType_Item, __Sumtype_TypeParam_0, __Sumtype_TypeParam_1,
__Sumtype_TypeParam_2 > :: core :: iter :: Iterator <> for
__Sumtype_Enum_8231541105705577287 <__Sumtype_TypeParam_0,
__Sumtype_TypeParam_1, __Sumtype_TypeParam_2 > where __Sumtype_TypeParam_0 :
:: core :: iter :: Iterator < Item = __SumType_AssocType_Item >,
__Sumtype_TypeParam_1 : :: core :: iter :: Iterator < Item =
__SumType_AssocType_Item >, __Sumtype_TypeParam_2 : :: core :: iter ::
Iterator < Item = __SumType_AssocType_Item >,
{
    type Item = __SumType_AssocType_Item; fn next(& mut self) -> :: core ::
    option :: Option < Self :: Item >
    {
        match self
        {
            __Sumtype_Enum_8231541105705577287
            ::__SumType_Variant_0(__sumtrait_self_arg) =>
            <__Sumtype_TypeParam_0 as :: core :: iter :: Iterator <
            >>::next(__sumtrait_self_arg), __Sumtype_Enum_8231541105705577287
            ::__SumType_Variant_1(__sumtrait_self_arg) =>
            <__Sumtype_TypeParam_1 as :: core :: iter :: Iterator <
            >>::next(__sumtrait_self_arg), __Sumtype_Enum_8231541105705577287
            ::__SumType_Variant_2(__sumtrait_self_arg) =>
            <__Sumtype_TypeParam_2 as :: core :: iter :: Iterator <
            >>::next(__sumtrait_self_arg), Self :: __Uninhabited(_) => :: core
            :: unreachable! (),
        }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_traits"
            && e.input == ""
            && e.to == r#"#[doc(hidden)] pub struct Marker(:: core :: convert :: Infallible);
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
            #[sumtrait(implement = :: std :: io :: Read, krate = $crate, marker = $crate
            :: traits :: Marker)] #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
                io :: Result < :: core :: primitive :: usize > ;
            } impl < T : :: std :: io :: Read > Read for T
            {
                fn read(& mut self, buf : & mut [u8]) -> std :: io :: Result < usize >
                { T :: read(self, buf) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`].
            #[sumtrait(implement = :: core :: iter :: Iterator, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Iterator
            {
                type Item; fn next(& mut self) -> :: core :: option :: Option < Self ::
                Item > ;
            } impl < T : :: core :: iter :: Iterator > Iterator for T
            {
                type Item = T :: Item; fn next(& mut self) -> Option < Self :: Item >
                { T :: next(self) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Copy`].
            #[sumtrait(implement = :: core :: marker :: Copy, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Copy : $crate
            :: traits :: Clone {} impl < T : :: core :: marker :: Copy > Copy for T {}
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Clone`].
            #[sumtrait(implement = :: core :: clone :: Clone, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Clone
            { fn clone(& self) -> Self; } impl < T : :: core :: clone :: Clone > Clone for
            T { fn clone(& self) -> Self { T :: clone(self) } }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Display`].
            #[sumtrait(implement = :: core :: fmt :: Display, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Display
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result;
            } impl < T : :: core :: fmt :: Display > Display for T
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result { T :: fmt(self, f) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Debug`].
            #[sumtrait(implement = :: core :: fmt :: Debug, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Debug
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result;
            } impl < T : :: core :: fmt :: Debug > Debug for T
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result { T :: fmt(self, f) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::error::Error`].
            #[sumtrait(implement = :: std :: error :: Error, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Error : $crate
            :: traits :: Debug + $crate :: traits :: Display
            {
                fn source(& self) -> :: core :: option :: Option < &
                (dyn :: std :: error :: Error + 'static)> ;
            } impl < T : :: std :: error :: Error > Error for T
            {
                fn source(& self) -> :: core :: option :: Option < &
                (dyn :: std :: error :: Error + 'static)> { T :: source(self) }
            }"#
    }));
}

#[test]
fn external_crate_sumtype_test_read() {
    let expansions = run_trace_for_repo("sumtype", Some("read"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && e.input == r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#
            && e.to == r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_13105404314811431591
{
    ($ ($t : tt) *) =>
    {
        $crate :: _sumtrait_internal!
        ({ $ ($t) * } [],
        {
            #[doc =
            " Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
            #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8])
                -> :: std :: io :: Result < :: core :: primitive :: usize > ;
            }
        }, 10781646948910720492usize, $crate, $crate :: traits :: Marker,
        [:: std :: io :: Read], [], [],);
    };
} #[doc(hidden)] pub use __sumtype_macro_13105404314811431591 as Read;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtype"
            && e.input == r#"#[allow(unused)] fn f1(a: bool) -> impl Read
{
    if a { sumtype!(std::io::empty()) } else
    { sumtype!(std::io::Cursor::new([1, 2, 3])) }
}"#
            && e.to == r#"#[doc(hidden)] #[allow(non_camel_case_types)] trait
__Sumtype_TypeRef_Trait_13413473787840962550 <> { type Type; } #[doc(hidden)]
#[allow(non_camel_case_types)] enum __Sumtype_Enum_13413473787840962550 <
__Sumtype_TypeParam_0, __Sumtype_TypeParam_1 >
{
    __SumType_Variant_0(__Sumtype_TypeParam_0),
    __SumType_Variant_1(__Sumtype_TypeParam_1),
    __Uninhabited((:: core :: convert :: Infallible,)),
} #[doc(hidden)] #[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_13413473787840962550 <> {} impl
<__Sumtype_TypeParam > __Sumtype_ConstraintExprTrait_13413473787840962550 <>
for __Sumtype_TypeParam where __Sumtype_TypeParam :
__Sumtype_ConstraintExprTrait_0_14044379981680716303 <>, {} sumtype :: traits
:: Read!
(__Sumtype_ConstraintExprTrait_0_14044379981680716303, sumtype :: traits ::
Read, __Sumtype_Enum_13413473787840962550,
[__Sumtype_TypeParam_0, __Sumtype_TypeParam_1],
[__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
:__Sumtype_TypeParam_1], [], [], {},); #[allow(non_local_definitions)]
#[allow(unused)] fn f1(a : bool) -> impl Read
{
    if a
    {
        {
            fn __sum_type_id_fn_5194699931172305089 < __SumType_T :
            __Sumtype_ConstraintExprTrait_13413473787840962550 < > >
            (t : __SumType_T) -> __SumType_T { t }
            __sum_type_id_fn_5194699931172305089 :: < _ >
            (__Sumtype_Enum_13413473787840962550 ::
            __SumType_Variant_0(std :: io :: empty()))
        }
    } else
    {
        {
            fn __sum_type_id_fn_245723284565521908 < __SumType_T :
            __Sumtype_ConstraintExprTrait_13413473787840962550 < > >
            (t : __SumType_T) -> __SumType_T { t }
            __sum_type_id_fn_245723284565521908 :: < _ >
            (__Sumtype_Enum_13413473787840962550 ::
            __SumType_Variant_1(std :: io :: Cursor :: new([1, 2, 3])))
        }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__sumtype_macro_13105404314811431591"
            && e.input == r#"__Sumtype_ConstraintExprTrait_0_14044379981680716303, sumtype :: traits ::
          Read, __Sumtype_Enum_13413473787840962550,
          [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1],
          [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
          :__Sumtype_TypeParam_1], [], [], {},"#
            && e.to == r#"$crate :: _sumtrait_internal!
          ({
              __Sumtype_ConstraintExprTrait_0_14044379981680716303, sumtype :: traits ::
              Read, __Sumtype_Enum_13413473787840962550,
              [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1],
              [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
              :__Sumtype_TypeParam_1], [], [], {},
          } [],
          {
              #[doc =
              " Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
              #[allow(private_bounds)] pub trait Read
              {
                  fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> ::
                  std :: io :: Result < :: core :: primitive :: usize > ;
              }
          }, 10781646948910720492usize, $crate, $crate :: traits :: Marker,
          [:: std :: io :: Read], [], [],);"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "_sumtrait_internal"
            && e.input == r#"{
    __Sumtype_ConstraintExprTrait_0_14044379981680716303, sumtype :: traits ::
    Read, __Sumtype_Enum_13413473787840962550,
    [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1],
    [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
    :__Sumtype_TypeParam_1], [], [], {},
} [],
{
    #[doc =
    " Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
    #[allow(private_bounds)] pub trait Read
    {
        fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> ::
        std :: io :: Result < :: core :: primitive :: usize > ;
    }
}, 10781646948910720492usize, $crate, $crate :: traits :: Marker,
[:: std :: io :: Read], [], [],"#
            && e.to == r#"#[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_0_14044379981680716303 <> {} impl
<__Sumtype_TypeParam_0, __Sumtype_TypeParam_1 >
__Sumtype_ConstraintExprTrait_0_14044379981680716303 <> for
__Sumtype_Enum_13413473787840962550 <__Sumtype_TypeParam_0,
__Sumtype_TypeParam_1 > where __Sumtype_TypeParam_0 : :: std :: io :: Read <
>, __Sumtype_TypeParam_1 : :: std :: io :: Read < >, {} impl
<__Sumtype_TypeParam_0, __Sumtype_TypeParam_1 > :: std :: io :: Read <> for
__Sumtype_Enum_13413473787840962550 <__Sumtype_TypeParam_0,
__Sumtype_TypeParam_1 > where __Sumtype_TypeParam_0 : :: std :: io :: Read <
>, __Sumtype_TypeParam_1 : :: std :: io :: Read < >,
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize >
    {
        match self
        {
            __Sumtype_Enum_13413473787840962550
            ::__SumType_Variant_0(__sumtrait_self_arg) =>
            <__Sumtype_TypeParam_0 as :: std :: io :: Read <
            >>::read(__sumtrait_self_arg, buf),
            __Sumtype_Enum_13413473787840962550
            ::__SumType_Variant_1(__sumtrait_self_arg) =>
            <__Sumtype_TypeParam_1 as :: std :: io :: Read <
            >>::read(__sumtrait_self_arg, buf), Self :: __Uninhabited(_) => ::
            core :: unreachable! (),
        }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_traits"
            && e.input == ""
            && e.to == r#"#[doc(hidden)] pub struct Marker(:: core :: convert :: Infallible);
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
            #[sumtrait(implement = :: std :: io :: Read, krate = $crate, marker = $crate
            :: traits :: Marker)] #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
                io :: Result < :: core :: primitive :: usize > ;
            } impl < T : :: std :: io :: Read > Read for T
            {
                fn read(& mut self, buf : & mut [u8]) -> std :: io :: Result < usize >
                { T :: read(self, buf) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`].
            #[sumtrait(implement = :: core :: iter :: Iterator, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Iterator
            {
                type Item; fn next(& mut self) -> :: core :: option :: Option < Self ::
                Item > ;
            } impl < T : :: core :: iter :: Iterator > Iterator for T
            {
                type Item = T :: Item; fn next(& mut self) -> Option < Self :: Item >
                { T :: next(self) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Copy`].
            #[sumtrait(implement = :: core :: marker :: Copy, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Copy : $crate
            :: traits :: Clone {} impl < T : :: core :: marker :: Copy > Copy for T {}
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Clone`].
            #[sumtrait(implement = :: core :: clone :: Clone, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Clone
            { fn clone(& self) -> Self; } impl < T : :: core :: clone :: Clone > Clone for
            T { fn clone(& self) -> Self { T :: clone(self) } }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Display`].
            #[sumtrait(implement = :: core :: fmt :: Display, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Display
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result;
            } impl < T : :: core :: fmt :: Display > Display for T
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result { T :: fmt(self, f) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Debug`].
            #[sumtrait(implement = :: core :: fmt :: Debug, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Debug
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result;
            } impl < T : :: core :: fmt :: Debug > Debug for T
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result { T :: fmt(self, f) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::error::Error`].
            #[sumtrait(implement = :: std :: error :: Error, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Error : $crate
            :: traits :: Debug + $crate :: traits :: Display
            {
                fn source(& self) -> :: core :: option :: Option < &
                (dyn :: std :: error :: Error + 'static)> ;
            } impl < T : :: std :: error :: Error > Error for T
            {
                fn source(& self) -> :: core :: option :: Option < &
                (dyn :: std :: error :: Error + 'static)> { T :: source(self) }
            }"#
    }));
}

#[test]
fn external_crate_sumtype_test_sumtrait() {
    let expansions = run_trace_for_repo("sumtype", Some("sumtrait"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && e.input == r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#
            && e.to == r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_13105404314811431591
{
    ($ ($t : tt) *) =>
    {
        $crate :: _sumtrait_internal!
        ({ $ ($t) * } [],
        {
            #[doc =
            " Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
            #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8])
                -> :: std :: io :: Result < :: core :: primitive :: usize > ;
            }
        }, 10781646948910720492usize, $crate, $crate :: traits :: Marker,
        [:: std :: io :: Read], [], [],);
    };
} #[doc(hidden)] pub use __sumtype_macro_13105404314811431591 as Read;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtype"
            && e.input == r#"#[allow(unused)] fn f1(a: bool) -> impl MySumTrait + Clone
{
    #[derive(Clone)] struct S1; #[derive(Clone)] struct S2; impl MySumTrait
    for S1 {} impl MySumTrait for S2 {} if a { sumtype!(S1) } else
    { sumtype!(S2) }
}"#
            && e.to == r#"#[doc(hidden)] #[allow(non_camel_case_types)] trait
__Sumtype_TypeRef_Trait_2413130888767878136 <> { type Type; } #[doc(hidden)]
#[allow(non_camel_case_types)] enum __Sumtype_Enum_2413130888767878136 <
__Sumtype_TypeParam_0, __Sumtype_TypeParam_1 >
{
    __SumType_Variant_0(__Sumtype_TypeParam_0),
    __SumType_Variant_1(__Sumtype_TypeParam_1),
    __Uninhabited((:: core :: convert :: Infallible,)),
} #[doc(hidden)] #[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_2413130888767878136 <> {} impl
<__Sumtype_TypeParam > __Sumtype_ConstraintExprTrait_2413130888767878136 <>
for __Sumtype_TypeParam where __Sumtype_TypeParam :
__Sumtype_ConstraintExprTrait_0_1948603960421176998 <>, {} MySumTrait!
(__Sumtype_ConstraintExprTrait_0_1948603960421176998, MySumTrait,
__Sumtype_Enum_2413130888767878136,
[__Sumtype_TypeParam_0, __Sumtype_TypeParam_1],
[__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
:__Sumtype_TypeParam_1], [], [], {},); #[allow(non_local_definitions)]
#[allow(unused)] fn f1(a : bool) -> impl MySumTrait + Clone
{
    #[derive(Clone)] struct S1; #[derive(Clone)] struct S2; impl MySumTrait
    for S1 {} impl MySumTrait for S2 {} if a
    {
        {
            fn __sum_type_id_fn_1346402187227292263 < __SumType_T :
            __Sumtype_ConstraintExprTrait_2413130888767878136 < > >
            (t : __SumType_T) -> __SumType_T { t }
            __sum_type_id_fn_1346402187227292263 :: < _ >
            (__Sumtype_Enum_2413130888767878136 :: __SumType_Variant_0(S1))
        }
    } else
    {
        {
            fn __sum_type_id_fn_6234339754159034000 < __SumType_T :
            __Sumtype_ConstraintExprTrait_2413130888767878136 < > >
            (t : __SumType_T) -> __SumType_T { t }
            __sum_type_id_fn_6234339754159034000 :: < _ >
            (__Sumtype_Enum_2413130888767878136 :: __SumType_Variant_1(S2))
        }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__sumtype_macro_9570217715475709253"
            && e.input == r#"__Sumtype_ConstraintExprTrait_0_6233570981542608572, MyCopy,
           __Sumtype_Enum_312545921254889825,
           [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1],
           [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
           :__Sumtype_TypeParam_1], [], [], {},"#
            && e.to == r#":: sumtype :: _sumtrait_internal!
           ({
               __Sumtype_ConstraintExprTrait_0_6233570981542608572, MyCopy,
               __Sumtype_Enum_312545921254889825,
               [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1],
               [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
               :__Sumtype_TypeParam_1], [], [], {},
           } [], { trait MyCopy : sumtype :: traits :: Copy {} },
           6700805951402385390usize, :: sumtype, Marker, [_],
           [sumtype :: traits :: Copy], [],);"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__sumtype_macro_4521917695666249494"
            && e.input == r#"__SumTrait_ConstraintTrait_0_6790641146150853326, MyCopy,
           __Sumtype_Enum_312545921254889825,
           [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1],
           [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
           :__Sumtype_TypeParam_1], [], [], {},"#
            && e.to == r#"$crate :: _sumtrait_internal!
           ({
               __SumTrait_ConstraintTrait_0_6790641146150853326, MyCopy,
               __Sumtype_Enum_312545921254889825,
               [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1],
               [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
               :__Sumtype_TypeParam_1], [], [], {},
           } [],
           {
               #[doc =
               " Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Copy`]."]
               #[allow(private_bounds)] pub trait Copy : $crate :: traits :: Clone {}
           }, 9179514995247523134usize, $crate, $crate :: traits :: Marker,
           [:: core :: marker :: Copy], [$crate :: traits :: Clone], [],);"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__sumtype_macro_11395927097724701696"
            && e.input == r#"__SumTrait_ConstraintTrait_0_10105587190761715128, MyCopy,
           __Sumtype_Enum_312545921254889825,
           [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1],
           [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
           :__Sumtype_TypeParam_1], [], [], {},"#
            && e.to == r#"$crate :: _sumtrait_internal!
           ({
               __SumTrait_ConstraintTrait_0_10105587190761715128, MyCopy,
               __Sumtype_Enum_312545921254889825,
               [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1],
               [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
               :__Sumtype_TypeParam_1], [], [], {},
           } [],
           {
               #[doc =
               " Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Clone`]."]
               #[allow(private_bounds)] pub trait Clone { fn clone(& self) -> Self; }
           }, 342573295450118012usize, $crate, $crate :: traits :: Marker,
           [:: core :: clone :: Clone], [], [],);"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__sumtype_macro_5127116426568902852"
            && e.input == r#"__Sumtype_ConstraintExprTrait_0_1948603960421176998, MySumTrait,
           __Sumtype_Enum_2413130888767878136,
           [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1],
           [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
           :__Sumtype_TypeParam_1], [], [], {},"#
            && e.to == r#":: sumtype :: _sumtrait_internal!
           ({
               __Sumtype_ConstraintExprTrait_0_1948603960421176998, MySumTrait,
               __Sumtype_Enum_2413130888767878136,
               [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1],
               [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
               :__Sumtype_TypeParam_1], [], [], {},
           } [], { trait MySumTrait : sumtype :: traits :: Clone {} },
           17904343677088984257usize, :: sumtype, Marker, [_],
           [sumtype :: traits :: Clone], [],);"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "_sumtrait_internal"
            && e.input == r#"{
    __Sumtype_ConstraintExprTrait_0_6233570981542608572, MyCopy,
    __Sumtype_Enum_312545921254889825,
    [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1],
    [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
    :__Sumtype_TypeParam_1], [], [], {},
} [], { trait MyCopy : sumtype :: traits :: Copy {} },
6700805951402385390usize, :: sumtype, Marker, [_],
[sumtype :: traits :: Copy], [],"#
            && e.to == r#"sumtype :: traits :: Copy!
(__SumTrait_ConstraintTrait_0_6790641146150853326, MyCopy,
__Sumtype_Enum_312545921254889825,
[__Sumtype_TypeParam_0, __Sumtype_TypeParam_1],
[__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
:__Sumtype_TypeParam_1], [], [], {},); #[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_0_6233570981542608572 <> {} impl
<__Sumtype_TypeParam_0, __Sumtype_TypeParam_1 >
__Sumtype_ConstraintExprTrait_0_6233570981542608572 <> for
__Sumtype_Enum_312545921254889825 <__Sumtype_TypeParam_0,
__Sumtype_TypeParam_1 > where __Sumtype_TypeParam_0 : MyCopy < >,
__Sumtype_TypeParam_1 : MyCopy < >, Self :
__SumTrait_ConstraintTrait_0_6790641146150853326 <>, {} impl
<__Sumtype_TypeParam_0, __Sumtype_TypeParam_1 > MyCopy <> for
__Sumtype_Enum_312545921254889825 <__Sumtype_TypeParam_0,
__Sumtype_TypeParam_1 > where __Sumtype_TypeParam_0 : MyCopy < >,
__Sumtype_TypeParam_1 : MyCopy < >, Self : sumtype :: traits :: Copy, {}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_traits"
            && e.input == ""
            && e.to == r#"#[doc(hidden)] pub struct Marker(:: core :: convert :: Infallible);
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
            #[sumtrait(implement = :: std :: io :: Read, krate = $crate, marker = $crate
            :: traits :: Marker)] #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
                io :: Result < :: core :: primitive :: usize > ;
            } impl < T : :: std :: io :: Read > Read for T
            {
                fn read(& mut self, buf : & mut [u8]) -> std :: io :: Result < usize >
                { T :: read(self, buf) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`].
            #[sumtrait(implement = :: core :: iter :: Iterator, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Iterator
            {
                type Item; fn next(& mut self) -> :: core :: option :: Option < Self ::
                Item > ;
            } impl < T : :: core :: iter :: Iterator > Iterator for T
            {
                type Item = T :: Item; fn next(& mut self) -> Option < Self :: Item >
                { T :: next(self) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Copy`].
            #[sumtrait(implement = :: core :: marker :: Copy, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Copy : $crate
            :: traits :: Clone {} impl < T : :: core :: marker :: Copy > Copy for T {}
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Clone`].
            #[sumtrait(implement = :: core :: clone :: Clone, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Clone
            { fn clone(& self) -> Self; } impl < T : :: core :: clone :: Clone > Clone for
            T { fn clone(& self) -> Self { T :: clone(self) } }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Display`].
            #[sumtrait(implement = :: core :: fmt :: Display, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Display
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result;
            } impl < T : :: core :: fmt :: Display > Display for T
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result { T :: fmt(self, f) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Debug`].
            #[sumtrait(implement = :: core :: fmt :: Debug, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Debug
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result;
            } impl < T : :: core :: fmt :: Debug > Debug for T
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result { T :: fmt(self, f) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::error::Error`].
            #[sumtrait(implement = :: std :: error :: Error, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Error : $crate
            :: traits :: Debug + $crate :: traits :: Display
            {
                fn source(& self) -> :: core :: option :: Option < &
                (dyn :: std :: error :: Error + 'static)> ;
            } impl < T : :: std :: error :: Error > Error for T
            {
                fn source(& self) -> :: core :: option :: Option < &
                (dyn :: std :: error :: Error + 'static)> { T :: source(self) }
            }"#
    }));
}

#[test]
fn external_crate_sumtype_test_test() {
    let expansions = run_trace_for_repo("sumtype", Some("test"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && e.input == r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#
            && e.to == r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_13105404314811431591
{
    ($ ($t : tt) *) =>
    {
        $crate :: _sumtrait_internal!
        ({ $ ($t) * } [],
        {
            #[doc =
            " Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
            #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8])
                -> :: std :: io :: Result < :: core :: primitive :: usize > ;
            }
        }, 10781646948910720492usize, $crate, $crate :: traits :: Marker,
        [:: std :: io :: Read], [], [],);
    };
} #[doc(hidden)] pub use __sumtype_macro_13105404314811431591 as Read;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtype"
            && e.input == r#"fn generate_iter<'a, T>(t: &'a T, count: usize) -> impl Iterator<Item = &'a T>
{
    match count
    {
        0 => sumtype!(std::iter::empty()), 1 => sumtype!(std::iter::once(t)),
        n => sumtype!(std::iter::repeat(t).take(n)),
    }
}"#
            && e.to == r#"#[doc(hidden)] #[allow(non_camel_case_types)] trait
__Sumtype_TypeRef_Trait_16716705739999546228 <'a, T > { type Type; }
#[doc(hidden)] #[allow(non_camel_case_types)] enum
__Sumtype_Enum_16716705739999546228 < 'a, T, __Sumtype_TypeParam_0,
__Sumtype_TypeParam_1, __Sumtype_TypeParam_2 >
{
    __SumType_Variant_0(__Sumtype_TypeParam_0),
    __SumType_Variant_1(__Sumtype_TypeParam_1),
    __SumType_Variant_2(__Sumtype_TypeParam_2),
    __Uninhabited((:: core :: convert :: Infallible, :: core :: marker ::
    PhantomData <& 'a () > , :: core :: marker :: PhantomData <T >)),
} #[doc(hidden)] #[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_16716705739999546228 <'a, T > {} impl <'a, T,
__Sumtype_TypeParam > __Sumtype_ConstraintExprTrait_16716705739999546228 <'a,
T > for __Sumtype_TypeParam where __Sumtype_TypeParam :
__Sumtype_ConstraintExprTrait_0_5129199822348944855 <'a, T >, {} sumtype ::
traits :: Iterator!
(__Sumtype_ConstraintExprTrait_0_5129199822348944855, sumtype :: traits ::
Iterator, __Sumtype_Enum_16716705739999546228,
[__Sumtype_TypeParam_0, __Sumtype_TypeParam_1, __Sumtype_TypeParam_2],
[__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
:__Sumtype_TypeParam_1, __SumType_Variant_2 :__Sumtype_TypeParam_2], ['a, T],
['a, T], {},); #[allow(non_local_definitions)] fn generate_iter < 'a, T >
(t : & 'a T, count : usize) -> impl Iterator < Item = & 'a T >
{
    match count
    {
        0 =>
        {
            fn __sum_type_id_fn_8561759151956293320 < 'a, T, __SumType_T :
            __Sumtype_ConstraintExprTrait_16716705739999546228 < 'a, T > >
            (t : __SumType_T) -> __SumType_T { t }
            __sum_type_id_fn_8561759151956293320 :: < 'a, T, _ >
            (__Sumtype_Enum_16716705739999546228 ::
            __SumType_Variant_0(std :: iter :: empty()))
        }, 1 =>
        {
            fn __sum_type_id_fn_17128884357753863759 < 'a, T, __SumType_T :
            __Sumtype_ConstraintExprTrait_16716705739999546228 < 'a, T > >
            (t : __SumType_T) -> __SumType_T { t }
            __sum_type_id_fn_17128884357753863759 :: < 'a, T, _ >
            (__Sumtype_Enum_16716705739999546228 ::
            __SumType_Variant_1(std :: iter :: once(t)))
        }, n =>
        {
            fn __sum_type_id_fn_6206647804032553929 < 'a, T, __SumType_T :
            __Sumtype_ConstraintExprTrait_16716705739999546228 < 'a, T > >
            (t : __SumType_T) -> __SumType_T { t }
            __sum_type_id_fn_6206647804032553929 :: < 'a, T, _ >
            (__Sumtype_Enum_16716705739999546228 ::
            __SumType_Variant_2(std :: iter :: repeat(t).take(n)))
        },
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__sumtype_macro_1521959044408027343"
            && e.input == r#"__Sumtype_ConstraintExprTrait_0_5129199822348944855, sumtype :: traits ::
          Iterator, __Sumtype_Enum_16716705739999546228,
          [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1, __Sumtype_TypeParam_2],
          [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
          :__Sumtype_TypeParam_1, __SumType_Variant_2 :__Sumtype_TypeParam_2], ['a, T],
          ['a, T], {},"#
            && e.to == r#"$crate :: _sumtrait_internal!
          ({
              __Sumtype_ConstraintExprTrait_0_5129199822348944855, sumtype :: traits ::
              Iterator, __Sumtype_Enum_16716705739999546228,
              [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1, __Sumtype_TypeParam_2],
              [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
              :__Sumtype_TypeParam_1, __SumType_Variant_2 :__Sumtype_TypeParam_2],
              ['a, T], ['a, T], {},
          } [],
          {
              #[doc =
              " Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`]."]
              #[allow(private_bounds)] pub trait Iterator
              {
                  type Item; fn next(& mut self) -> :: core :: option :: Option < Self
                  :: Item > ;
              }
          }, 3440346043459666793usize, $crate, $crate :: traits :: Marker,
          [:: core :: iter :: Iterator], [], [],);"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "_sumtrait_internal"
            && e.input == r#"{
    __Sumtype_ConstraintExprTrait_0_5129199822348944855, sumtype :: traits ::
    Iterator, __Sumtype_Enum_16716705739999546228,
    [__Sumtype_TypeParam_0, __Sumtype_TypeParam_1, __Sumtype_TypeParam_2],
    [__SumType_Variant_0 :__Sumtype_TypeParam_0, __SumType_Variant_1
    :__Sumtype_TypeParam_1, __SumType_Variant_2 :__Sumtype_TypeParam_2],
    ['a, T], ['a, T], {},
} [],
{
    #[doc =
    " Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`]."]
    #[allow(private_bounds)] pub trait Iterator
    {
        type Item; fn next(& mut self) -> :: core :: option :: Option < Self
        :: Item > ;
    }
}, 3440346043459666793usize, $crate, $crate :: traits :: Marker,
[:: core :: iter :: Iterator], [], [],"#
            && e.to == r#"#[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_0_5129199822348944855 <'a, T > {} impl <'a, T,
__SumType_AssocType_Item, __Sumtype_TypeParam_0, __Sumtype_TypeParam_1,
__Sumtype_TypeParam_2 > __Sumtype_ConstraintExprTrait_0_5129199822348944855
<'a, T > for __Sumtype_Enum_16716705739999546228 <'a, T,
__Sumtype_TypeParam_0, __Sumtype_TypeParam_1, __Sumtype_TypeParam_2 > where
__Sumtype_TypeParam_0 : :: core :: iter :: Iterator < Item =
__SumType_AssocType_Item >, __Sumtype_TypeParam_1 : :: core :: iter ::
Iterator < Item = __SumType_AssocType_Item >, __Sumtype_TypeParam_2 : :: core
:: iter :: Iterator < Item = __SumType_AssocType_Item >, {} impl <'a, T,
__SumType_AssocType_Item, __Sumtype_TypeParam_0, __Sumtype_TypeParam_1,
__Sumtype_TypeParam_2 > :: core :: iter :: Iterator <> for
__Sumtype_Enum_16716705739999546228 <'a, T, __Sumtype_TypeParam_0,
__Sumtype_TypeParam_1, __Sumtype_TypeParam_2 > where __Sumtype_TypeParam_0 :
:: core :: iter :: Iterator < Item = __SumType_AssocType_Item >,
__Sumtype_TypeParam_1 : :: core :: iter :: Iterator < Item =
__SumType_AssocType_Item >, __Sumtype_TypeParam_2 : :: core :: iter ::
Iterator < Item = __SumType_AssocType_Item >,
{
    type Item = __SumType_AssocType_Item; fn next(& mut self) -> :: core ::
    option :: Option < Self :: Item >
    {
        match self
        {
            __Sumtype_Enum_16716705739999546228
            ::__SumType_Variant_0(__sumtrait_self_arg) =>
            <__Sumtype_TypeParam_0 as :: core :: iter :: Iterator <
            >>::next(__sumtrait_self_arg), __Sumtype_Enum_16716705739999546228
            ::__SumType_Variant_1(__sumtrait_self_arg) =>
            <__Sumtype_TypeParam_1 as :: core :: iter :: Iterator <
            >>::next(__sumtrait_self_arg), __Sumtype_Enum_16716705739999546228
            ::__SumType_Variant_2(__sumtrait_self_arg) =>
            <__Sumtype_TypeParam_2 as :: core :: iter :: Iterator <
            >>::next(__sumtrait_self_arg), Self :: __Uninhabited(_) => :: core
            :: unreachable! (),
        }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_traits"
            && e.input == ""
            && e.to == r#"#[doc(hidden)] pub struct Marker(:: core :: convert :: Infallible);
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
            #[sumtrait(implement = :: std :: io :: Read, krate = $crate, marker = $crate
            :: traits :: Marker)] #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
                io :: Result < :: core :: primitive :: usize > ;
            } impl < T : :: std :: io :: Read > Read for T
            {
                fn read(& mut self, buf : & mut [u8]) -> std :: io :: Result < usize >
                { T :: read(self, buf) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`].
            #[sumtrait(implement = :: core :: iter :: Iterator, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Iterator
            {
                type Item; fn next(& mut self) -> :: core :: option :: Option < Self ::
                Item > ;
            } impl < T : :: core :: iter :: Iterator > Iterator for T
            {
                type Item = T :: Item; fn next(& mut self) -> Option < Self :: Item >
                { T :: next(self) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Copy`].
            #[sumtrait(implement = :: core :: marker :: Copy, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Copy : $crate
            :: traits :: Clone {} impl < T : :: core :: marker :: Copy > Copy for T {}
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Clone`].
            #[sumtrait(implement = :: core :: clone :: Clone, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Clone
            { fn clone(& self) -> Self; } impl < T : :: core :: clone :: Clone > Clone for
            T { fn clone(& self) -> Self { T :: clone(self) } }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Display`].
            #[sumtrait(implement = :: core :: fmt :: Display, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Display
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result;
            } impl < T : :: core :: fmt :: Display > Display for T
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result { T :: fmt(self, f) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Debug`].
            #[sumtrait(implement = :: core :: fmt :: Debug, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Debug
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result;
            } impl < T : :: core :: fmt :: Debug > Debug for T
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result { T :: fmt(self, f) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::error::Error`].
            #[sumtrait(implement = :: std :: error :: Error, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Error : $crate
            :: traits :: Debug + $crate :: traits :: Display
            {
                fn source(& self) -> :: core :: option :: Option < &
                (dyn :: std :: error :: Error + 'static)> ;
            } impl < T : :: std :: error :: Error > Error for T
            {
                fn source(& self) -> :: core :: option :: Option < &
                (dyn :: std :: error :: Error + 'static)> { T :: source(self) }
            }"#
    }));
}

#[test]
fn external_crate_sumtype_test_test_gparams() {
    let expansions = run_trace_for_repo("sumtype", Some("test_gparams"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && e.input == r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#
            && e.to == r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_13105404314811431591
{
    ($ ($t : tt) *) =>
    {
        $crate :: _sumtrait_internal!
        ({ $ ($t) * } [],
        {
            #[doc =
            " Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
            #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8])
                -> :: std :: io :: Result < :: core :: primitive :: usize > ;
            }
        }, 10781646948910720492usize, $crate, $crate :: traits :: Marker,
        [:: std :: io :: Read], [], [],);
    };
} #[doc(hidden)] pub use __sumtype_macro_13105404314811431591 as Read;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtype"
            && e.input == r#"#[allow(unused)] fn with_generics<'a, T>(t: &'a T, count: usize) -> sumtype!()
{
    match count
    {
        0 => sumtype!(std::iter::empty(), std::iter::Empty<&'a T>), 1 =>
        sumtype!(std::iter::once(t), std::iter::Once<&'a T>), n =>
        sumtype!(std::iter::repeat(t).take(n),
        std::iter::Take<std::iter::Repeat<&'a T>>),
    }
}"#
            && e.to == r#"#[doc(hidden)] #[allow(non_camel_case_types)] #[allow(non_camel_case_types)]
struct __SumType_RefType_1062508501731093496_0; #[doc(hidden)]
#[allow(non_camel_case_types)] #[allow(non_camel_case_types)] struct
__SumType_RefType_18286400472473946005_1; #[doc(hidden)]
#[allow(non_camel_case_types)] #[allow(non_camel_case_types)] struct
__SumType_RefType_4325181398777167502_2; #[doc(hidden)]
#[allow(non_camel_case_types)] trait
__Sumtype_TypeRef_Trait_6248564123702680449 <'a, T : 'a > { type Type; }
#[doc(hidden)] #[allow(non_camel_case_types)] enum
__Sumtype_Enum_6248564123702680449 < 'a, T : 'a >
{
    __SumType_Variant_0(< __SumType_RefType_1062508501731093496_0 as
    __Sumtype_TypeRef_Trait_6248564123702680449 < 'a, T > > :: Type),
    __SumType_Variant_1(< __SumType_RefType_18286400472473946005_1 as
    __Sumtype_TypeRef_Trait_6248564123702680449 < 'a, T > > :: Type),
    __SumType_Variant_2(< __SumType_RefType_4325181398777167502_2 as
    __Sumtype_TypeRef_Trait_6248564123702680449 < 'a, T > > :: Type),
    __Uninhabited((:: core :: convert :: Infallible, :: core :: marker ::
    PhantomData <& 'a () > , :: core :: marker :: PhantomData <T >)),
} #[doc(hidden)] #[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_6248564123702680449 <'a, T : 'a > {} impl <'a, T
: 'a, __Sumtype_TypeParam > __Sumtype_ConstraintExprTrait_6248564123702680449
<'a, T > for __Sumtype_TypeParam where __Sumtype_TypeParam :
__Sumtype_ConstraintExprTrait_0_8877389768854752063 <'a, T >, {} sumtype ::
traits :: Iterator!
(__Sumtype_ConstraintExprTrait_0_8877389768854752063, sumtype :: traits ::
Iterator, __Sumtype_Enum_6248564123702680449, [],
[__SumType_Variant_0 :< __SumType_RefType_1062508501731093496_0 as
__Sumtype_TypeRef_Trait_6248564123702680449 < 'a, T > > :: Type,
__SumType_Variant_1 :< __SumType_RefType_18286400472473946005_1 as
__Sumtype_TypeRef_Trait_6248564123702680449 < 'a, T > > :: Type,
__SumType_Variant_2 :< __SumType_RefType_4325181398777167502_2 as
__Sumtype_TypeRef_Trait_6248564123702680449 < 'a, T > > :: Type],
['a, T : 'a], ['a, T], {},); #[allow(non_local_definitions)] #[allow(unused)]
fn with_generics < 'a, T > (t : & 'a T, count : usize) ->
__Sumtype_Enum_6248564123702680449 < 'a, T >
{
    match count
    {
        0 =>
        {
            impl < 'a, T : 'a, > __Sumtype_TypeRef_Trait_6248564123702680449 <
            'a, T > for __SumType_RefType_1062508501731093496_0
            { type Type = std :: iter :: Empty < & 'a T > ; } fn
            __sum_type_id_fn_9240388630947599046 < 'a, T : 'a, __SumType_T :
            __Sumtype_ConstraintExprTrait_6248564123702680449 < 'a, T > >
            (t : __SumType_T) -> __SumType_T { t }
            __sum_type_id_fn_9240388630947599046 :: < 'a, T, _ >
            (__Sumtype_Enum_6248564123702680449 ::
            __SumType_Variant_0(std :: iter :: empty()))
        }, 1 =>
        {
            impl < 'a, T : 'a, > __Sumtype_TypeRef_Trait_6248564123702680449 <
            'a, T > for __SumType_RefType_18286400472473946005_1
            { type Type = std :: iter :: Once < & 'a T > ; } fn
            __sum_type_id_fn_12901230919275950126 < 'a, T : 'a, __SumType_T :
            __Sumtype_ConstraintExprTrait_6248564123702680449 < 'a, T > >
            (t : __SumType_T) -> __SumType_T { t }
            __sum_type_id_fn_12901230919275950126 :: < 'a, T, _ >
            (__Sumtype_Enum_6248564123702680449 ::
            __SumType_Variant_1(std :: iter :: once(t)))
        }, n =>
        {
            impl < 'a, T : 'a, > __Sumtype_TypeRef_Trait_6248564123702680449 <
            'a, T > for __SumType_RefType_4325181398777167502_2
            {
                type Type = std :: iter :: Take < std :: iter :: Repeat < & 'a
                T > > ;
            } fn __sum_type_id_fn_8721820607653119248 < 'a, T : 'a,
            __SumType_T : __Sumtype_ConstraintExprTrait_6248564123702680449 <
            'a, T > > (t : __SumType_T) -> __SumType_T { t }
            __sum_type_id_fn_8721820607653119248 :: < 'a, T, _ >
            (__Sumtype_Enum_6248564123702680449 ::
            __SumType_Variant_2(std :: iter :: repeat(t).take(n)))
        },
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__sumtype_macro_1521959044408027343"
            && e.input == r#"__Sumtype_ConstraintExprTrait_0_8877389768854752063, sumtype :: traits ::
          Iterator, __Sumtype_Enum_6248564123702680449, [],
          [__SumType_Variant_0 :< __SumType_RefType_1062508501731093496_0 as
          __Sumtype_TypeRef_Trait_6248564123702680449 < 'a, T > > :: Type,
          __SumType_Variant_1 :< __SumType_RefType_18286400472473946005_1 as
          __Sumtype_TypeRef_Trait_6248564123702680449 < 'a, T > > :: Type,
          __SumType_Variant_2 :< __SumType_RefType_4325181398777167502_2 as
          __Sumtype_TypeRef_Trait_6248564123702680449 < 'a, T > > :: Type],
          ['a, T : 'a], ['a, T], {},"#
            && e.to == r#"$crate :: _sumtrait_internal!
          ({
              __Sumtype_ConstraintExprTrait_0_8877389768854752063, sumtype :: traits ::
              Iterator, __Sumtype_Enum_6248564123702680449, [],
              [__SumType_Variant_0 :< __SumType_RefType_1062508501731093496_0 as
              __Sumtype_TypeRef_Trait_6248564123702680449 < 'a, T > > :: Type,
              __SumType_Variant_1 :< __SumType_RefType_18286400472473946005_1 as
              __Sumtype_TypeRef_Trait_6248564123702680449 < 'a, T > > :: Type,
              __SumType_Variant_2 :< __SumType_RefType_4325181398777167502_2 as
              __Sumtype_TypeRef_Trait_6248564123702680449 < 'a, T > > :: Type],
              ['a, T : 'a], ['a, T], {},
          } [],
          {
              #[doc =
              " Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`]."]
              #[allow(private_bounds)] pub trait Iterator
              {
                  type Item; fn next(& mut self) -> :: core :: option :: Option < Self
                  :: Item > ;
              }
          }, 3440346043459666793usize, $crate, $crate :: traits :: Marker,
          [:: core :: iter :: Iterator], [], [],);"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "_sumtrait_internal"
            && e.input == r#"{
    __Sumtype_ConstraintExprTrait_0_8877389768854752063, sumtype :: traits ::
    Iterator, __Sumtype_Enum_6248564123702680449, [],
    [__SumType_Variant_0 :< __SumType_RefType_1062508501731093496_0 as
    __Sumtype_TypeRef_Trait_6248564123702680449 < 'a, T > > :: Type,
    __SumType_Variant_1 :< __SumType_RefType_18286400472473946005_1 as
    __Sumtype_TypeRef_Trait_6248564123702680449 < 'a, T > > :: Type,
    __SumType_Variant_2 :< __SumType_RefType_4325181398777167502_2 as
    __Sumtype_TypeRef_Trait_6248564123702680449 < 'a, T > > :: Type],
    ['a, T : 'a], ['a, T], {},
} [],
{
    #[doc =
    " Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`]."]
    #[allow(private_bounds)] pub trait Iterator
    {
        type Item; fn next(& mut self) -> :: core :: option :: Option < Self
        :: Item > ;
    }
}, 3440346043459666793usize, $crate, $crate :: traits :: Marker,
[:: core :: iter :: Iterator], [], [],"#
            && e.to == r#"#[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_0_8877389768854752063 <'a, T : 'a > {} impl <'a,
T : 'a, __SumType_AssocType_Item >
__Sumtype_ConstraintExprTrait_0_8877389768854752063 <'a, T > for
__Sumtype_Enum_6248564123702680449 <'a, T > where <
__SumType_RefType_1062508501731093496_0 as
__Sumtype_TypeRef_Trait_6248564123702680449 < 'a, T > > :: Type : :: core ::
iter :: Iterator < Item = __SumType_AssocType_Item >, <
__SumType_RefType_18286400472473946005_1 as
__Sumtype_TypeRef_Trait_6248564123702680449 < 'a, T > > :: Type : :: core ::
iter :: Iterator < Item = __SumType_AssocType_Item >, <
__SumType_RefType_4325181398777167502_2 as
__Sumtype_TypeRef_Trait_6248564123702680449 < 'a, T > > :: Type : :: core ::
iter :: Iterator < Item = __SumType_AssocType_Item >, {} impl <'a, T : 'a,
__SumType_AssocType_Item > :: core :: iter :: Iterator <> for
__Sumtype_Enum_6248564123702680449 <'a, T > where <
__SumType_RefType_1062508501731093496_0 as
__Sumtype_TypeRef_Trait_6248564123702680449 < 'a, T > > :: Type : :: core ::
iter :: Iterator < Item = __SumType_AssocType_Item >, <
__SumType_RefType_18286400472473946005_1 as
__Sumtype_TypeRef_Trait_6248564123702680449 < 'a, T > > :: Type : :: core ::
iter :: Iterator < Item = __SumType_AssocType_Item >, <
__SumType_RefType_4325181398777167502_2 as
__Sumtype_TypeRef_Trait_6248564123702680449 < 'a, T > > :: Type : :: core ::
iter :: Iterator < Item = __SumType_AssocType_Item >,
{
    type Item = __SumType_AssocType_Item; fn next(& mut self) -> :: core ::
    option :: Option < Self :: Item >
    {
        match self
        {
            __Sumtype_Enum_6248564123702680449
            ::__SumType_Variant_0(__sumtrait_self_arg) => <<
            __SumType_RefType_1062508501731093496_0 as
            __Sumtype_TypeRef_Trait_6248564123702680449 < 'a, T > > :: Type as
            :: core :: iter :: Iterator < >>::next(__sumtrait_self_arg),
            __Sumtype_Enum_6248564123702680449
            ::__SumType_Variant_1(__sumtrait_self_arg) => <<
            __SumType_RefType_18286400472473946005_1 as
            __Sumtype_TypeRef_Trait_6248564123702680449 < 'a, T > > :: Type as
            :: core :: iter :: Iterator < >>::next(__sumtrait_self_arg),
            __Sumtype_Enum_6248564123702680449
            ::__SumType_Variant_2(__sumtrait_self_arg) => <<
            __SumType_RefType_4325181398777167502_2 as
            __Sumtype_TypeRef_Trait_6248564123702680449 < 'a, T > > :: Type as
            :: core :: iter :: Iterator < >>::next(__sumtrait_self_arg), Self
            :: __Uninhabited(_) => :: core :: unreachable! (),
        }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_traits"
            && e.input == ""
            && e.to == r#"#[doc(hidden)] pub struct Marker(:: core :: convert :: Infallible);
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
            #[sumtrait(implement = :: std :: io :: Read, krate = $crate, marker = $crate
            :: traits :: Marker)] #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
                io :: Result < :: core :: primitive :: usize > ;
            } impl < T : :: std :: io :: Read > Read for T
            {
                fn read(& mut self, buf : & mut [u8]) -> std :: io :: Result < usize >
                { T :: read(self, buf) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`].
            #[sumtrait(implement = :: core :: iter :: Iterator, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Iterator
            {
                type Item; fn next(& mut self) -> :: core :: option :: Option < Self ::
                Item > ;
            } impl < T : :: core :: iter :: Iterator > Iterator for T
            {
                type Item = T :: Item; fn next(& mut self) -> Option < Self :: Item >
                { T :: next(self) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Copy`].
            #[sumtrait(implement = :: core :: marker :: Copy, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Copy : $crate
            :: traits :: Clone {} impl < T : :: core :: marker :: Copy > Copy for T {}
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Clone`].
            #[sumtrait(implement = :: core :: clone :: Clone, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Clone
            { fn clone(& self) -> Self; } impl < T : :: core :: clone :: Clone > Clone for
            T { fn clone(& self) -> Self { T :: clone(self) } }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Display`].
            #[sumtrait(implement = :: core :: fmt :: Display, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Display
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result;
            } impl < T : :: core :: fmt :: Display > Display for T
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result { T :: fmt(self, f) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Debug`].
            #[sumtrait(implement = :: core :: fmt :: Debug, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Debug
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result;
            } impl < T : :: core :: fmt :: Debug > Debug for T
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result { T :: fmt(self, f) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::error::Error`].
            #[sumtrait(implement = :: std :: error :: Error, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Error : $crate
            :: traits :: Debug + $crate :: traits :: Display
            {
                fn source(& self) -> :: core :: option :: Option < &
                (dyn :: std :: error :: Error + 'static)> ;
            } impl < T : :: std :: error :: Error > Error for T
            {
                fn source(& self) -> :: core :: option :: Option < &
                (dyn :: std :: error :: Error + 'static)> { T :: source(self) }
            }"#
    }));
}

#[test]
fn external_crate_sumtype_test_test_mod() {
    let expansions = run_trace_for_repo("sumtype", Some("test_mod"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && e.input == r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#
            && e.to == r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_13105404314811431591
{
    ($ ($t : tt) *) =>
    {
        $crate :: _sumtrait_internal!
        ({ $ ($t) * } [],
        {
            #[doc =
            " Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
            #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8])
                -> :: std :: io :: Result < :: core :: primitive :: usize > ;
            }
        }, 10781646948910720492usize, $crate, $crate :: traits :: Marker,
        [:: std :: io :: Read], [], [],);
    };
} #[doc(hidden)] pub use __sumtype_macro_13105404314811431591 as Read;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtype"
            && e.input == r#"mod my_module
{
    #[allow(unused)] pub struct MyStruct { iter: sumtype!(), } impl MyStruct
    {
        #[allow(unused)] pub fn new(flag: bool) -> Self
        {
            let iter = if flag { sumtype!(0..5, std::ops::Range<u32>) } else
            {
                sumtype!(vec![10, 20, 30].into_iter(),
                std::vec::IntoIter<u32>)
            }; MyStruct { iter }
        } #[allow(unused)] pub fn iterate(self)
        { for value in self.iter { println!("{}", value); } }
    }
}"#
            && e.to == r#"#[doc(hidden)] #[allow(non_camel_case_types)] #[allow(non_camel_case_types)]
struct __SumType_RefType_1918033710817013100_0; #[doc(hidden)]
#[allow(non_camel_case_types)] #[allow(non_camel_case_types)] struct
__SumType_RefType_12109140973942214259_1; #[doc(hidden)]
#[allow(non_camel_case_types)] trait
__Sumtype_TypeRef_Trait_141863029148527706 <> { type Type; } #[doc(hidden)]
#[allow(non_camel_case_types)] pub enum __Sumtype_Enum_141863029148527706 < >
{
    __SumType_Variant_0(< __SumType_RefType_1918033710817013100_0 as
    __Sumtype_TypeRef_Trait_141863029148527706 < > > :: Type),
    __SumType_Variant_1(< __SumType_RefType_12109140973942214259_1 as
    __Sumtype_TypeRef_Trait_141863029148527706 < > > :: Type),
    __Uninhabited((:: core :: convert :: Infallible,)),
} #[doc(hidden)] #[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_141863029148527706 <> {} impl
<__Sumtype_TypeParam > __Sumtype_ConstraintExprTrait_141863029148527706 <> for
__Sumtype_TypeParam where __Sumtype_TypeParam :
__Sumtype_ConstraintExprTrait_0_10758456767235873351 <>, {} sumtype :: traits
:: Iterator!
(__Sumtype_ConstraintExprTrait_0_10758456767235873351, sumtype :: traits ::
Iterator, __Sumtype_Enum_141863029148527706, [],
[__SumType_Variant_0 :< __SumType_RefType_1918033710817013100_0 as
__Sumtype_TypeRef_Trait_141863029148527706 < > > :: Type, __SumType_Variant_1
:< __SumType_RefType_12109140973942214259_1 as
__Sumtype_TypeRef_Trait_141863029148527706 < > > :: Type], [], [], {},);
#[allow(non_local_definitions)] mod my_module
{
    #[allow(unused)] pub struct MyStruct
    { iter : super :: __Sumtype_Enum_141863029148527706, } impl MyStruct
    {
        #[allow(unused)] pub fn new(flag : bool) -> Self
        {
            let iter = if flag
            {
                {
                    impl super :: __Sumtype_TypeRef_Trait_141863029148527706 < >
                    for super :: __SumType_RefType_1918033710817013100_0
                    { type Type = std :: ops :: Range < u32 > ; } fn
                    __sum_type_id_fn_4169030566704044545 < __SumType_T : super
                    :: __Sumtype_ConstraintExprTrait_141863029148527706 < > >
                    (t : __SumType_T) -> __SumType_T { t }
                    __sum_type_id_fn_4169030566704044545 :: < _ >
                    (super :: __Sumtype_Enum_141863029148527706 ::
                    __SumType_Variant_0(0 .. 5))
                }
            } else
            {
                {
                    impl super :: __Sumtype_TypeRef_Trait_141863029148527706 < >
                    for super :: __SumType_RefType_12109140973942214259_1
                    { type Type = std :: vec :: IntoIter < u32 > ; } fn
                    __sum_type_id_fn_17645757429626402495 < __SumType_T : super
                    :: __Sumtype_ConstraintExprTrait_141863029148527706 < > >
                    (t : __SumType_T) -> __SumType_T { t }
                    __sum_type_id_fn_17645757429626402495 :: < _ >
                    (super :: __Sumtype_Enum_141863029148527706 ::
                    __SumType_Variant_1(vec! [10, 20, 30].into_iter()))
                }
            }; MyStruct { iter }
        } #[allow(unused)] pub fn iterate(self)
        { for value in self.iter { println! ("{}", value); } }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__sumtype_macro_1521959044408027343"
            && e.input == r#"__Sumtype_ConstraintExprTrait_0_10758456767235873351, sumtype :: traits ::
          Iterator, __Sumtype_Enum_141863029148527706, [],
          [__SumType_Variant_0 :< __SumType_RefType_1918033710817013100_0 as
          __Sumtype_TypeRef_Trait_141863029148527706 < > > :: Type, __SumType_Variant_1
          :< __SumType_RefType_12109140973942214259_1 as
          __Sumtype_TypeRef_Trait_141863029148527706 < > > :: Type], [], [], {},"#
            && e.to == r#"$crate :: _sumtrait_internal!
          ({
              __Sumtype_ConstraintExprTrait_0_10758456767235873351, sumtype :: traits ::
              Iterator, __Sumtype_Enum_141863029148527706, [],
              [__SumType_Variant_0 :< __SumType_RefType_1918033710817013100_0 as
              __Sumtype_TypeRef_Trait_141863029148527706 < > > :: Type,
              __SumType_Variant_1 :< __SumType_RefType_12109140973942214259_1 as
              __Sumtype_TypeRef_Trait_141863029148527706 < > > :: Type], [], [], {},
          } [],
          {
              #[doc =
              " Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`]."]
              #[allow(private_bounds)] pub trait Iterator
              {
                  type Item; fn next(& mut self) -> :: core :: option :: Option < Self
                  :: Item > ;
              }
          }, 3440346043459666793usize, $crate, $crate :: traits :: Marker,
          [:: core :: iter :: Iterator], [], [],);"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "_sumtrait_internal"
            && e.input == r#"{
    __Sumtype_ConstraintExprTrait_0_10758456767235873351, sumtype :: traits ::
    Iterator, __Sumtype_Enum_141863029148527706, [],
    [__SumType_Variant_0 :< __SumType_RefType_1918033710817013100_0 as
    __Sumtype_TypeRef_Trait_141863029148527706 < > > :: Type,
    __SumType_Variant_1 :< __SumType_RefType_12109140973942214259_1 as
    __Sumtype_TypeRef_Trait_141863029148527706 < > > :: Type], [], [], {},
} [],
{
    #[doc =
    " Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`]."]
    #[allow(private_bounds)] pub trait Iterator
    {
        type Item; fn next(& mut self) -> :: core :: option :: Option < Self
        :: Item > ;
    }
}, 3440346043459666793usize, $crate, $crate :: traits :: Marker,
[:: core :: iter :: Iterator], [], [],"#
            && e.to == r#"#[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_0_10758456767235873351 <> {} impl
<__SumType_AssocType_Item >
__Sumtype_ConstraintExprTrait_0_10758456767235873351 <> for
__Sumtype_Enum_141863029148527706 <> where <
__SumType_RefType_1918033710817013100_0 as
__Sumtype_TypeRef_Trait_141863029148527706 < > > :: Type : :: core :: iter ::
Iterator < Item = __SumType_AssocType_Item >, <
__SumType_RefType_12109140973942214259_1 as
__Sumtype_TypeRef_Trait_141863029148527706 < > > :: Type : :: core :: iter ::
Iterator < Item = __SumType_AssocType_Item >, {} impl
<__SumType_AssocType_Item > :: core :: iter :: Iterator <> for
__Sumtype_Enum_141863029148527706 <> where <
__SumType_RefType_1918033710817013100_0 as
__Sumtype_TypeRef_Trait_141863029148527706 < > > :: Type : :: core :: iter ::
Iterator < Item = __SumType_AssocType_Item >, <
__SumType_RefType_12109140973942214259_1 as
__Sumtype_TypeRef_Trait_141863029148527706 < > > :: Type : :: core :: iter ::
Iterator < Item = __SumType_AssocType_Item >,
{
    type Item = __SumType_AssocType_Item; fn next(& mut self) -> :: core ::
    option :: Option < Self :: Item >
    {
        match self
        {
            __Sumtype_Enum_141863029148527706
            ::__SumType_Variant_0(__sumtrait_self_arg) => <<
            __SumType_RefType_1918033710817013100_0 as
            __Sumtype_TypeRef_Trait_141863029148527706 < > > :: Type as ::
            core :: iter :: Iterator < >>::next(__sumtrait_self_arg),
            __Sumtype_Enum_141863029148527706
            ::__SumType_Variant_1(__sumtrait_self_arg) => <<
            __SumType_RefType_12109140973942214259_1 as
            __Sumtype_TypeRef_Trait_141863029148527706 < > > :: Type as ::
            core :: iter :: Iterator < >>::next(__sumtrait_self_arg), Self ::
            __Uninhabited(_) => :: core :: unreachable! (),
        }
    }
}"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_traits"
            && e.input == ""
            && e.to == r#"#[doc(hidden)] pub struct Marker(:: core :: convert :: Infallible);
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
            #[sumtrait(implement = :: std :: io :: Read, krate = $crate, marker = $crate
            :: traits :: Marker)] #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
                io :: Result < :: core :: primitive :: usize > ;
            } impl < T : :: std :: io :: Read > Read for T
            {
                fn read(& mut self, buf : & mut [u8]) -> std :: io :: Result < usize >
                { T :: read(self, buf) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`].
            #[sumtrait(implement = :: core :: iter :: Iterator, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Iterator
            {
                type Item; fn next(& mut self) -> :: core :: option :: Option < Self ::
                Item > ;
            } impl < T : :: core :: iter :: Iterator > Iterator for T
            {
                type Item = T :: Item; fn next(& mut self) -> Option < Self :: Item >
                { T :: next(self) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Copy`].
            #[sumtrait(implement = :: core :: marker :: Copy, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Copy : $crate
            :: traits :: Clone {} impl < T : :: core :: marker :: Copy > Copy for T {}
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Clone`].
            #[sumtrait(implement = :: core :: clone :: Clone, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Clone
            { fn clone(& self) -> Self; } impl < T : :: core :: clone :: Clone > Clone for
            T { fn clone(& self) -> Self { T :: clone(self) } }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Display`].
            #[sumtrait(implement = :: core :: fmt :: Display, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Display
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result;
            } impl < T : :: core :: fmt :: Display > Display for T
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result { T :: fmt(self, f) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Debug`].
            #[sumtrait(implement = :: core :: fmt :: Debug, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Debug
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result;
            } impl < T : :: core :: fmt :: Debug > Debug for T
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result { T :: fmt(self, f) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::error::Error`].
            #[sumtrait(implement = :: std :: error :: Error, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Error : $crate
            :: traits :: Debug + $crate :: traits :: Display
            {
                fn source(& self) -> :: core :: option :: Option < &
                (dyn :: std :: error :: Error + 'static)> ;
            } impl < T : :: std :: error :: Error > Error for T
            {
                fn source(& self) -> :: core :: option :: Option < &
                (dyn :: std :: error :: Error + 'static)> { T :: source(self) }
            }"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "println"
            && e.input == r#""{}", value"#
            && e.to == r#"{ $crate :: io :: _print($crate :: format_args_nl! ("{}", value)); }"#
    }));
}

#[test]
fn external_crate_sumtype_test_ui() {
    let expansions = run_trace_for_repo("sumtype", Some("ui"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && e.input == r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#
            && e.to == r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_13105404314811431591
{
    ($ ($t : tt) *) =>
    {
        $crate :: _sumtrait_internal!
        ({ $ ($t) * } [],
        {
            #[doc =
            " Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
            #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8])
                -> :: std :: io :: Result < :: core :: primitive :: usize > ;
            }
        }, 10781646948910720492usize, $crate, $crate :: traits :: Marker,
        [:: std :: io :: Read], [], [],);
    };
} #[doc(hidden)] pub use __sumtype_macro_13105404314811431591 as Read;"#
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_traits"
            && e.input == ""
            && e.to == r#"#[doc(hidden)] pub struct Marker(:: core :: convert :: Infallible);
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
            #[sumtrait(implement = :: std :: io :: Read, krate = $crate, marker = $crate
            :: traits :: Marker)] #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
                io :: Result < :: core :: primitive :: usize > ;
            } impl < T : :: std :: io :: Read > Read for T
            {
                fn read(& mut self, buf : & mut [u8]) -> std :: io :: Result < usize >
                { T :: read(self, buf) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`].
            #[sumtrait(implement = :: core :: iter :: Iterator, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Iterator
            {
                type Item; fn next(& mut self) -> :: core :: option :: Option < Self ::
                Item > ;
            } impl < T : :: core :: iter :: Iterator > Iterator for T
            {
                type Item = T :: Item; fn next(& mut self) -> Option < Self :: Item >
                { T :: next(self) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Copy`].
            #[sumtrait(implement = :: core :: marker :: Copy, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Copy : $crate
            :: traits :: Clone {} impl < T : :: core :: marker :: Copy > Copy for T {}
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::marker::Clone`].
            #[sumtrait(implement = :: core :: clone :: Clone, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Clone
            { fn clone(& self) -> Self; } impl < T : :: core :: clone :: Clone > Clone for
            T { fn clone(& self) -> Self { T :: clone(self) } }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Display`].
            #[sumtrait(implement = :: core :: fmt :: Display, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Display
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result;
            } impl < T : :: core :: fmt :: Display > Display for T
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result { T :: fmt(self, f) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::fmt::Debug`].
            #[sumtrait(implement = :: core :: fmt :: Debug, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Debug
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result;
            } impl < T : :: core :: fmt :: Debug > Debug for T
            {
                fn fmt(& self, f : & mut :: core :: fmt :: Formatter < '_ >) -> :: core ::
                fmt :: Result { T :: fmt(self, f) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::error::Error`].
            #[sumtrait(implement = :: std :: error :: Error, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Error : $crate
            :: traits :: Debug + $crate :: traits :: Display
            {
                fn source(& self) -> :: core :: option :: Option < &
                (dyn :: std :: error :: Error + 'static)> ;
            } impl < T : :: std :: error :: Error > Error for T
            {
                fn source(& self) -> :: core :: option :: Option < &
                (dyn :: std :: error :: Error + 'static)> { T :: source(self) }
            }"#
    }));
}

