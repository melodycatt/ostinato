#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    // ── Test EntityType implementations ─────────────────────────────────────

    struct Foo(i32);
    impl EntityType for Foo {}

    struct Bar(String);
    impl EntityType for Bar {}

    /// Lets us observe whether drop was called
    struct Tracked(Arc<Mutex<Vec<&'static str>>>, &'static str);
    impl EntityType for Tracked {}
    impl Drop for Tracked {
        fn drop(&mut self) {
            self.0.lock().unwrap().push(self.1);
        }
    }

    // / EntityType where DowncastType differs from Self
    struct Wrapper;
    struct Inner(u64);
    impl EntityType for Wrapper {
        type DowncastType = Inner;
    }
    // spawn_marker::<Wrapper> requires value: Wrapper; but ptr stores Inner.
    // We test this path by keeping DowncastType=Self for most tests and only
    // verifying the type check uses TypeId::of::<T>() vs TypeId::of::<T::DowncastType>().

    // ── Helpers ─────────────────────────────────────────────────────────────

    fn drop_log() -> Arc<Mutex<Vec<&'static str>>> {
        Arc::new(Mutex::new(Vec::new()))
    }

    // ── Handle ───────────────────────────────────────────────────────────────

    #[test]
    fn handle_index_and_generation() {
        let h = Handle::new(42, 7);
        assert_eq!(h.index(), 42);
        assert_eq!(h.generation(), 7);
    }

    #[test]
    fn handle_increment_changes_generation_not_index() {
        let mut h = Handle::new(100, 0);
        h.increment();
        assert_eq!(h.index(), 100);
        assert_eq!(h.generation(), 1);
        h.increment();
        assert_eq!(h.generation(), 2);
    }

    #[test]
    fn handle_new_round_trips() {
        let h = Handle::new(0xDEAD, 0xBEEF);
        assert_eq!(h.index(), 0xDEAD);
        assert_eq!(h.generation(), 0xBEEF & 0xFFFFF); // generation is 20 bits
    }

    // ── Spawn / entity retrieval ─────────────────────────────────────────────

    #[test]
    fn spawn_returns_live_handle() {
        let mut w = World::new();
        let h = w.spawn(Foo(1));
        assert!(w.entity(h).is_ok());
    }

    #[test]
    fn spawn_multiple_unique_handles() {
        let mut w = World::new();
        let h1 = w.spawn(Foo(1));
        let h2 = w.spawn(Foo(2));
        let h3 = w.spawn(Bar("x".into()));
        assert_ne!(h1, h2);
        assert_ne!(h1, h3);
        assert_ne!(h2, h3);
    }

    #[test]
    fn entity_mut_round_trips() {
        let mut w = World::new();
        let h = w.spawn(Foo(99));
        let e = w.entity_mut(h).unwrap();
        assert_eq!(e.downcast::<Foo>().unwrap().0, 99);
    }

    #[test]
    fn spawned_entity_is_self_parented() {
        let mut w = World::new();
        let h = w.spawn(Foo(0));
        let e = w.entity(h).unwrap();
        assert_eq!(e.parent, h);
    }

    #[test]
    fn spawned_entity_has_no_children() {
        let mut w = World::new();
        let h = w.spawn(Foo(0));
        assert!(w.entity(h).unwrap().children.is_empty());
    }

    // ── Downcast ─────────────────────────────────────────────────────────────

    #[test]
    fn downcast_correct_type() {
        let mut w = World::new();
        let h = w.spawn(Foo(42));
        let v = w.entity(h).unwrap().downcast::<Foo>().unwrap();
        assert_eq!(v.0, 42);
    }

    #[test]
    fn downcast_wrong_type_returns_error() {
        let mut w = World::new();
        let h = w.spawn(Foo(1));
        let err = w.entity(h).unwrap().downcast::<Bar>();
        assert!(matches!(err, Err(Error::WrongType(_))));
    }

    #[test]
    fn downcast_mut_modifies_value() {
        let mut w = World::new();
        let h = w.spawn(Foo(0));
        w.entity_mut(h).unwrap().downcast_mut::<Foo>().unwrap().0 = 77;
        assert_eq!(w.entity(h).unwrap().downcast::<Foo>().unwrap().0, 77);
    }

    // ── Kill ─────────────────────────────────────────────────────────────────

    #[test]
    fn kill_invalidates_handle() {
        let mut w = World::new();
        let h = w.spawn(Foo(1));
        w.kill(h).unwrap();
        assert!(matches!(w.entity(h), Err(Error::Dead)));
    }

    #[test]
    fn kill_drop_fn_is_called() {
        let log = drop_log();
        let mut w = World::new();
        let h = w.spawn(Tracked(log.clone(), "a"));
        assert!(log.lock().unwrap().is_empty());
        w.kill(h).unwrap();
        assert_eq!(*log.lock().unwrap(), vec!["a"]);
    }

    #[test]
    fn kill_dead_handle_returns_error() {
        let mut w = World::new();
        let h = w.spawn(Foo(1));
        w.kill(h).unwrap();
        assert!(matches!(w.kill(h), Err(Error::Dead)));
    }

    #[test]
    fn kill_recursively_drops_children() {
        let log = drop_log();
        let mut w = World::new();

        let parent = w.spawn(Tracked(log.clone(), "parent"));
        let child1 = w.spawn(Tracked(log.clone(), "child1"));
        let child2 = w.spawn(Tracked(log.clone(), "child2"));

        w.attach_child::<Tracked>(parent, child1).unwrap();
        w.attach_child::<Tracked>(parent, child2).unwrap();

        w.kill(parent).unwrap();

        let mut dropped = log.lock().unwrap().clone();
        dropped.sort();
        assert_eq!(dropped, vec!["child1", "child2", "parent"]);

        assert!(matches!(w.entity(parent), Err(Error::Dead)));
        assert!(matches!(w.entity(child1), Err(Error::Dead)));
        assert!(matches!(w.entity(child2), Err(Error::Dead)));
    }

    #[test]
    fn kill_deep_tree_drops_all() {
        let log = drop_log();
        let mut w = World::new();

        let root = w.spawn(Tracked(log.clone(), "root"));
        let mid = w.spawn(Tracked(log.clone(), "mid"));
        let leaf = w.spawn(Tracked(log.clone(), "leaf"));

        w.attach_child::<Tracked>(root, mid).unwrap();
        w.attach_child::<Tracked>(mid, leaf).unwrap();

        w.kill(root).unwrap();

        let mut dropped = log.lock().unwrap().clone();
        dropped.sort();
        assert_eq!(dropped, vec!["leaf", "mid", "root"]);
    }

    #[test]
    fn kill_child_detaches_from_parent() {
        let mut w = World::new();
        let parent = w.spawn(Foo(0));
        let child = w.spawn(Foo(1));
        w.attach_child::<Foo>(parent, child).unwrap();

        w.kill(child).unwrap();

        let pe = w.entity(parent).unwrap();
        assert!(pe.children.is_empty());
        assert!(pe.type_map.is_empty());
    }

    #[test]
    fn kill_removes_parent_from_world_types() {
        let mut w = World::new();
        let parent = w.spawn(Foo(0));
        let child = w.spawn(Foo(1));
        w.attach_child::<Foo>(parent, child).unwrap();

        let child_ty = TypeId::of::<Foo>();
        assert!(
            w.types
                .get(&child_ty)
                .map_or(false, |v| v.contains(&parent))
        );

        w.kill(child).unwrap();
        let still_listed = w
            .types
            .get(&child_ty)
            .map_or(false, |v| v.contains(&parent));
        assert!(
            !still_listed,
            "parent should be removed from world.types after last child killed"
        );
    }

    // ── Slot reuse after kill ────────────────────────────────────────────────

    #[test]
    fn slot_reused_after_kill_old_handle_stays_dead() {
        let mut w = World::new();
        let h1 = w.spawn(Foo(1));
        w.kill(h1).unwrap();

        let h2 = w.spawn(Foo(2)); // should reuse h1's slot
        assert_eq!(h2.index(), h1.index());
        assert_ne!(h2.generation(), h1.generation()); // generation must differ

        assert!(matches!(w.entity(h1), Err(Error::Dead)));
        assert!(w.entity(h2).is_ok());
    }

    // ── attach_child ─────────────────────────────────────────────────────────

    #[test]
    fn attach_child_basic() {
        let mut w = World::new();
        let parent = w.spawn(Foo(0));
        let child = w.spawn(Foo(1));

        let was_new = w.attach_child::<Foo>(parent, child).unwrap();
        assert!(was_new);

        let pe = w.entity(parent).unwrap();
        assert!(pe.children.contains(&child));
        assert_eq!(w.entity(child).unwrap().parent, parent);
    }

    #[test]
    fn attach_child_already_attached_returns_false() {
        let mut w = World::new();
        let parent = w.spawn(Foo(0));
        let child = w.spawn(Foo(1));

        w.attach_child::<Foo>(parent, child).unwrap();
        let was_new = w.attach_child::<Foo>(parent, child).unwrap();
        assert!(!was_new);
    }

    #[test]
    fn attach_child_updates_world_types() {
        let mut w = World::new();
        let parent = w.spawn(Foo(0));
        let child = w.spawn(Foo(1));
        w.attach_child::<Foo>(parent, child).unwrap();

        let ty = TypeId::of::<Foo>();
        assert!(w.types.get(&ty).map_or(false, |v| v.contains(&parent)));
    }

    #[test]
    fn attach_child_world_types_not_duplicated() {
        let mut w = World::new();
        let parent = w.spawn(Foo(0));
        let c1 = w.spawn(Foo(1));
        let c2 = w.spawn(Foo(2));
        w.attach_child::<Foo>(parent, c1).unwrap();
        w.attach_child::<Foo>(parent, c2).unwrap();

        let ty = TypeId::of::<Foo>();
        let count = w.types[&ty].iter().filter(|&&h| h == parent).count();
        assert_eq!(count, 1, "parent should appear at most once in world.types");
    }

    #[test]
    fn attach_child_reparents_correctly() {
        let mut w = World::new();
        let p1 = w.spawn(Foo(0));
        let p2 = w.spawn(Foo(1));
        let c = w.spawn(Foo(2));

        w.attach_child::<Foo>(p1, c).unwrap();
        w.attach_child::<Foo>(p2, c).unwrap(); // re-parent

        // c should be under p2 now
        assert_eq!(w.entity(c).unwrap().parent, p2);
        assert!(w.entity(p2).unwrap().children.contains(&c));

        // p1 should no longer have c
        assert!(!w.entity(p1).unwrap().children.contains(&c));
    }

    #[test]
    fn attach_child_reparent_cleans_old_parent_type_map() {
        let mut w = World::new();
        let p1 = w.spawn(Foo(0));
        let p2 = w.spawn(Foo(1));
        let c = w.spawn(Foo(2));

        w.attach_child::<Foo>(p1, c).unwrap();
        w.attach_child::<Foo>(p2, c).unwrap();

        let ty = TypeId::of::<Foo>();
        // p1 should no longer appear in world.types for Foo
        let p1_listed = w.types.get(&ty).map_or(false, |v| v.contains(&p1));
        assert!(!p1_listed);
    }

    #[test]
    fn attach_child_multiple_types_tracked_separately() {
        let mut w = World::new();
        let parent = w.spawn(Foo(0));
        let cf = w.spawn(Foo(1));
        let cb = w.spawn(Bar("x".into()));

        w.attach_child::<Foo>(parent, cf).unwrap();
        w.attach_child::<Bar>(parent, cb).unwrap();

        let foo_ty = TypeId::of::<Foo>();
        let bar_ty = TypeId::of::<Bar>();
        assert!(w.types.get(&foo_ty).map_or(false, |v| v.contains(&parent)));
        assert!(w.types.get(&bar_ty).map_or(false, |v| v.contains(&parent)));
    }

    #[test]
    fn attach_child_type_map_indices_correct() {
        let mut w = World::new();
        let parent = w.spawn(Foo(0));
        let c1 = w.spawn(Foo(1));
        let c2 = w.spawn(Bar("x".into()));
        let c3 = w.spawn(Foo(3));

        w.attach_child::<Foo>(parent, c1).unwrap(); // children[0]
        w.attach_child::<Bar>(parent, c2).unwrap(); // children[1]
        w.attach_child::<Foo>(parent, c3).unwrap(); // children[2]

        let pe = w.entity(parent).unwrap();
        let foo_indices = pe.type_map.get(&TypeId::of::<Foo>()).unwrap();
        let mut fi = foo_indices.clone();
        fi.sort();
        assert_eq!(fi, vec![0, 2]);

        let bar_indices = pe.type_map.get(&TypeId::of::<Bar>()).unwrap();
        assert_eq!(*bar_indices, vec![1]);
    }

    // ── Cycle detection ──────────────────────────────────────────────────────

    #[test]
    fn attach_child_direct_cycle_rejected() {
        let mut w = World::new();
        let a = w.spawn(Foo(0));
        let b = w.spawn(Foo(1));
        w.attach_child::<Foo>(a, b).unwrap();

        // trying to make b the parent of a would loop
        let err = w.attach_child::<Foo>(b, a);
        assert!(matches!(err, Err(Error::InvalidStructure)));
    }

    #[test]
    fn attach_child_indirect_cycle_rejected() {
        let mut w = World::new();
        let a = w.spawn(Foo(0));
        let b = w.spawn(Foo(1));
        let c = w.spawn(Foo(2));

        w.attach_child::<Foo>(a, b).unwrap(); // a → b
        w.attach_child::<Foo>(b, c).unwrap(); // b → c

        // c → a would create a → b → c → a loop
        let err = w.attach_child::<Foo>(c, a);
        assert!(matches!(err, Err(Error::InvalidStructure)));
    }

    #[test]
    fn attach_self_as_child_rejected() {
        // A self-parented root must not become its own child (immediate cycle)
        let mut w = World::new();
        let a = w.spawn(Foo(0));
        let err = w.attach_child::<Foo>(a, a);
        assert!(matches!(err, Err(Error::InvalidStructure)));
    }

    // ── remove_child ─────────────────────────────────────────────────────────

    #[test]
    fn remove_child_basic() {
        let mut w = World::new();
        let parent = w.spawn(Foo(0));
        let child = w.spawn(Foo(1));
        w.attach_child::<Foo>(parent, child).unwrap();

        w.remove_child(parent, child, TypeId::of::<Foo>()).unwrap();

        let pe = w.entity(parent).unwrap();
        assert!(!pe.children.contains(&child));
        assert!(
            pe.type_map
                .get(&TypeId::of::<Foo>())
                .map_or(true, |v| v.is_empty())
        );
        assert_eq!(w.entity(child).unwrap().parent, child); // child becomes root
    }

    #[test]
    fn remove_child_cleans_world_types_when_last() {
        let mut w = World::new();
        let parent = w.spawn(Foo(0));
        let child = w.spawn(Foo(1));
        w.attach_child::<Foo>(parent, child).unwrap();

        w.remove_child(parent, child, TypeId::of::<Foo>()).unwrap();

        let ty = TypeId::of::<Foo>();
        let still_listed = w.types.get(&ty).map_or(false, |v| v.contains(&parent));
        assert!(!still_listed);
    }

    #[test]
    fn remove_child_keeps_world_types_when_sibling_remains() {
        let mut w = World::new();
        let parent = w.spawn(Foo(0));
        let c1 = w.spawn(Foo(1));
        let c2 = w.spawn(Foo(2));
        w.attach_child::<Foo>(parent, c1).unwrap();
        w.attach_child::<Foo>(parent, c2).unwrap();

        w.remove_child(parent, c1, TypeId::of::<Foo>()).unwrap();

        let ty = TypeId::of::<Foo>();
        assert!(
            w.types.get(&ty).map_or(false, |v| v.contains(&parent)),
            "parent should stay in world.types while it still has a Foo child"
        );
    }

    #[test]
    fn remove_middle_child_swap_remove_fixup() {
        // attach 3 Foo children; remove the middle one.
        // The swap_remove moves children[2] → children[1].
        // The type_map index for the moved element must update from 2 → 1.
        let mut w = World::new();
        let parent = w.spawn(Foo(0));
        let c0 = w.spawn(Foo(10));
        let c1 = w.spawn(Foo(20));
        let c2 = w.spawn(Foo(30));

        w.attach_child::<Foo>(parent, c0).unwrap(); // children[0]
        w.attach_child::<Foo>(parent, c1).unwrap(); // children[1]
        w.attach_child::<Foo>(parent, c2).unwrap(); // children[2]

        w.remove_child(parent, c1, TypeId::of::<Foo>()).unwrap();

        let pe = w.entity(parent).unwrap();
        // After swap_remove of index 1, c2 lands at index 1
        assert_eq!(pe.children.len(), 2);
        assert!(pe.children.contains(&c0));
        assert!(pe.children.contains(&c2));
        assert!(!pe.children.contains(&c1));

        // type_map must reflect the new positions exactly
        let mut indices = pe.type_map[&TypeId::of::<Foo>()].clone();
        indices.sort();
        let expected: Vec<usize> = pe
            .children
            .iter()
            .enumerate()
            .filter(|&(_, &h)| h == c0 || h == c2)
            .map(|(i, _)| i)
            .collect();
        let mut expected = expected;
        expected.sort();
        assert_eq!(indices, expected);
    }

    #[test]
    fn remove_child_missing_returns_error() {
        let mut w = World::new();
        let parent = w.spawn(Foo(0));
        let stray = w.spawn(Foo(1));
        let err = w.remove_child(parent, stray, TypeId::of::<Foo>());
        assert!(matches!(err, Err(Error::Missing)));
    }

    #[test]
    fn remove_child_dead_parent_returns_error() {
        let mut w = World::new();
        let parent = w.spawn(Foo(0));
        let child = w.spawn(Foo(1));
        w.attach_child::<Foo>(parent, child).unwrap();
        // kill parent first — child's remove_child target is now dead
        // (we have to bypass the normal API; use the raw dead handle directly)
        let dead_parent = parent;
        w.kill(parent).unwrap();
        let err = w.remove_child(dead_parent, child, TypeId::of::<Foo>());
        assert!(matches!(err, Err(Error::Dead)));
    }

    // ── Dead handle rejection ────────────────────────────────────────────────

    #[test]
    fn entity_dead_handle_rejected() {
        let mut w = World::new();
        let h = w.spawn(Foo(1));
        w.kill(h).unwrap();
        assert!(matches!(w.entity(h), Err(Error::Dead)));
    }

    #[test]
    fn entity_mut_dead_handle_rejected() {
        let mut w = World::new();
        let h = w.spawn(Foo(1));
        w.kill(h).unwrap();
        assert!(matches!(w.entity_mut(h), Err(Error::Dead)));
    }

    #[test]
    fn stale_handle_after_slot_reuse_rejected() {
        let mut w = World::new();
        let h1 = w.spawn(Foo(1));
        w.kill(h1).unwrap();
        let h2 = w.spawn(Foo(2)); // reuses slot, higher generation
        assert_eq!(h2.index(), h1.index());
        assert!(matches!(w.entity(h1), Err(Error::Dead)));
        assert!(w.entity(h2).is_ok());
    }

    // ── Complex scenarios ────────────────────────────────────────────────────

    #[test]
    fn killing_sibling_does_not_corrupt_parent_type_map() {
        // parent has c1 and c2; kill c1; c2's index in type_map must still be valid
        let mut w = World::new();
        let parent = w.spawn(Foo(0));
        let c1 = w.spawn(Foo(10));
        let c2 = w.spawn(Foo(20));
        w.attach_child::<Foo>(parent, c1).unwrap();
        w.attach_child::<Foo>(parent, c2).unwrap();

        w.kill(c1).unwrap();

        let pe = w.entity(parent).unwrap();
        assert_eq!(pe.children.len(), 1);
        assert_eq!(pe.children[0], c2);

        let indices = pe.type_map.get(&TypeId::of::<Foo>()).unwrap();
        assert_eq!(*indices, vec![0]);
    }

    #[test]
    fn many_children_kill_then_query_consistent() {
        let log = drop_log();
        let mut w = World::new();
        let parent = w.spawn(Foo(0));

        let names = ["a", "b", "c", "d", "e"];
        let children: Vec<Handle> = names
            .iter()
            .map(|&n| {
                let h = w.spawn(Tracked(log.clone(), n));
                w.attach_child::<Tracked>(parent, h).unwrap();
                h
            })
            .collect();

        // Remove the middle child individually, then kill the parent
        w.kill(children[2]).unwrap(); // kills "c"

        let pe = w.entity(parent).unwrap();
        assert_eq!(pe.children.len(), 4);
        // type_map indices must all be in-bounds
        for &idx in pe.type_map.get(&TypeId::of::<Tracked>()).unwrap() {
            assert!(idx < pe.children.len());
        }

        w.kill(parent).unwrap();
        let mut dropped = log.lock().unwrap().clone();
        dropped.sort();
        assert_eq!(dropped, vec!["a", "b", "c", "d", "e"]);
    }

    #[test]
    fn mixed_type_children_correct_isolation() {
        let mut w = World::new();
        let parent = w.spawn(Foo(0));
        let f1 = w.spawn(Foo(1));
        let b1 = w.spawn(Bar("x".into()));
        let f2 = w.spawn(Foo(2));
        let b2 = w.spawn(Bar("y".into()));

        w.attach_child::<Foo>(parent, f1).unwrap();
        w.attach_child::<Bar>(parent, b1).unwrap();
        w.attach_child::<Foo>(parent, f2).unwrap();
        w.attach_child::<Bar>(parent, b2).unwrap();

        // Remove one Foo; Bar indices must be untouched
        let bar_indices_before = w.entity(parent).unwrap().type_map[&TypeId::of::<Bar>()].clone();

        w.remove_child(parent, f1, TypeId::of::<Foo>()).unwrap();

        let pe = w.entity(parent).unwrap();
        for &idx in pe.type_map.get(&TypeId::of::<Bar>()).unwrap() {
            assert!(
                idx < pe.children.len(),
                "Bar index out of bounds after removing a Foo"
            );
            // the handle at that index must actually be a Bar
            let h = pe.children[idx];
            assert!(h == b1 || h == b2);
        }
        // Bar count unchanged
        assert_eq!(
            pe.type_map[&TypeId::of::<Bar>()].len(),
            bar_indices_before.len()
        );
    }

    #[test]
    fn world_types_empty_after_all_parents_killed() {
        let mut w = World::new();
        let p = w.spawn(Foo(0));
        let c = w.spawn(Foo(1));
        w.attach_child::<Foo>(p, c).unwrap();
        w.kill(p).unwrap();

        let ty = TypeId::of::<Foo>();
        let leftover = w.types.get(&ty).map_or(0, |v| v.len());
        assert_eq!(leftover, 0);
    }

    #[test]
    fn reattach_to_original_parent_after_reparent() {
        let mut w = World::new();
        let p1 = w.spawn(Foo(0));
        let p2 = w.spawn(Foo(1));
        let c = w.spawn(Foo(2));

        w.attach_child::<Foo>(p1, c).unwrap();
        w.attach_child::<Foo>(p2, c).unwrap(); // p1 → p2
        w.attach_child::<Foo>(p1, c).unwrap(); // back to p1

        assert_eq!(w.entity(c).unwrap().parent, p1);
        assert!(w.entity(p1).unwrap().children.contains(&c));
        assert!(!w.entity(p2).unwrap().children.contains(&c));
    }
    // ── Bar(String) ──────────────────────────────────────────────────────────

    #[test]
    fn bar_downcast_reads_string() {
        let mut w = World::new();
        let h = w.spawn_marker::<Bar>(Bar("hello".into()));
        assert_eq!(w.entity(h).unwrap().downcast::<Bar>().unwrap().0, "hello");
    }

    #[test]
    fn bar_downcast_mut_modifies_string() {
        let mut w = World::new();
        let h = w.spawn_marker::<Bar>(Bar("before".into()));
        w.entity_mut(h).unwrap().downcast_mut::<Bar>().unwrap().0 = "after".into();
        assert_eq!(w.entity(h).unwrap().downcast::<Bar>().unwrap().0, "after");
    }

    #[test]
    fn bar_string_dropped_on_kill() {
        let mut w = World::new();
        let h = w.spawn_marker::<Bar>(Bar("x".repeat(10_000)));
        w.kill(h).unwrap();
        assert!(matches!(w.entity(h), Err(Error::Dead)));
    }

    // ── Wrapper / Inner (DowncastType != Self) ────────────────────────────────

    #[test]
    fn wrapper_downcast_reaches_inner() {
        let mut w = World::new();
        let h = w.spawn_marker::<Wrapper>(Inner(99));
        assert_eq!(w.entity(h).unwrap().downcast::<Wrapper>().unwrap().0, 99);
    }

    #[test]
    fn wrapper_downcast_mut_modifies_inner() {
        let mut w = World::new();
        let h = w.spawn_marker::<Wrapper>(Inner(0));
        w.entity_mut(h)
            .unwrap()
            .downcast_mut::<Wrapper>()
            .unwrap()
            .0 = 42;
        assert_eq!(w.entity(h).unwrap().downcast::<Wrapper>().unwrap().0, 42);
    }

    #[test]
    fn wrapper_wrong_type_rejected() {
        let mut w = World::new();
        let h = w.spawn_marker::<Wrapper>(Inner(1));
        assert!(matches!(
            w.entity(h).unwrap().downcast::<Foo>(),
            Err(Error::WrongType(_))
        ));
    }

    #[test]
    fn wrapper_ty_is_typeid_of_wrapper_not_inner() {
        // Pins the contract: entity.ty is the label type T, not T::DowncastType.
        // This is what makes Wrapper useful as a distinct child category from
        // any other EntityType that also uses Inner as its DowncastType.
        let mut w = World::new();
        let h = w.spawn_marker::<Wrapper>(Inner(0));
        let entity = w.entity(h).unwrap();
        assert_eq!(entity.ty, TypeId::of::<Wrapper>());
        assert_ne!(entity.ty, TypeId::of::<Inner>());
    }

    #[test]
    fn wrapper_killed_without_leak() {
        // drop_fn boxes as *mut Inner, which is what we allocated — no mismatch.
        let mut w = World::new();
        let h = w.spawn_marker::<Wrapper>(Inner(7));
        w.kill(h).unwrap();
        assert!(matches!(w.entity(h), Err(Error::Dead)));
    }

    #[test]
    fn wrapper_label_separates_from_foo_in_type_map() {
        // Both Wrapper and Foo children store Inner/Foo data, but they are
        // filed under different TypeIds so queries for one don't return the other.
        let mut w = World::new();
        let parent = w.spawn_marker::<Foo>(Foo(0));
        let wrapper = w.spawn_marker::<Wrapper>(Inner(1));
        let foo = w.spawn_marker::<Foo>(Foo(2));

        w.attach_child::<Wrapper>(parent, wrapper).unwrap();
        w.attach_child::<Foo>(parent, foo).unwrap();

        let pe = w.entity(parent).unwrap();
        assert_eq!(pe.type_map[&TypeId::of::<Wrapper>()].len(), 1);
        assert_eq!(pe.type_map[&TypeId::of::<Foo>()].len(), 1);

        // The slot the type_map points to is actually the wrapper handle
        let wrapper_idx = pe.type_map[&TypeId::of::<Wrapper>()][0];
        assert_eq!(pe.children[wrapper_idx], wrapper);
    }

    #[test]
    fn two_wrappers_same_parent_both_tracked() {
        let mut w = World::new();
        let parent = w.spawn_marker::<Foo>(Foo(0));
        let w1 = w.spawn_marker::<Wrapper>(Inner(10));
        let w2 = w.spawn_marker::<Wrapper>(Inner(20));

        w.attach_child::<Wrapper>(parent, w1).unwrap();
        w.attach_child::<Wrapper>(parent, w2).unwrap();

        let pe = w.entity(parent).unwrap();
        let indices = &pe.type_map[&TypeId::of::<Wrapper>()];
        assert_eq!(indices.len(), 2);
        let handles: Vec<Handle> = indices.iter().map(|&i| pe.children[i]).collect();
        assert!(handles.contains(&w1));
        assert!(handles.contains(&w2));
    }
}

use std::any::TypeId;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::ops::{Index, IndexMut};

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Handle(u64);
impl Handle {
    fn index(&self) -> usize {
        (self.0 as u32) as usize
    }
    fn generation(&self) -> u64 {
        (self.0 >> 32) & 0xFFFFF
    }
    fn with_generation(self, generation: u64) -> Self {
        Self((self.0 & !(0xFFFFFu64 << 32)) | (generation << 32))
    }
    fn increment(&mut self) {
        *self = self.with_generation(self.generation() + 1)
    }
    fn new(index: usize, generation: u64) -> Self {
        Self(index as u64 & 0xFFFFFFFF | generation << 32)
    }
}

impl<T> IndexMut<Handle> for Vec<T> {
    fn index_mut(&mut self, index: Handle) -> &mut Self::Output {
        &mut self[index.index()]
    }
}
impl<T> Index<Handle> for Vec<T> {
    type Output = T;
    fn index(&self, index: Handle) -> &Self::Output {
        &self[index.index()]
    }
}

#[derive(Debug)]
pub struct Entity {
    parent: Handle,
    handle: Handle,
    /// typeid of t (the entitytype implementor), not t::downcasttype
    ty: TypeId,
    ptr: *mut (),
    drop_fn: fn(*mut ()),
    children: Vec<Handle>,
    /// child typeid → indices into `self.children`
    type_map: HashMap<TypeId, Vec<usize>, BuildHasherDefault<TypeIdHasher>>,
}

impl Entity {
    pub fn downcast<T: EntityType>(&self) -> Result<&T::DowncastType, Error> {
        if self.ty != TypeId::of::<T>() {
            return Err(Error::WrongType(self.ty));
        }
        unsafe {
            (self.ptr as *mut T::DowncastType)
                .as_ref()
                .ok_or(Error::InvalidPtr)
        }
    }
    pub fn downcast_mut<T: EntityType>(&mut self) -> Result<&mut T::DowncastType, Error> {
        if self.ty != TypeId::of::<T>() {
            return Err(Error::WrongType(self.ty));
        }
        unsafe {
            (self.ptr as *mut T::DowncastType)
                .as_mut()
                .ok_or(Error::InvalidPtr)
        }
    }
}

pub struct World {
    free_handles: Vec<Handle>,
    /// child typeid → handles of parent entities that own ≥1 child of that type
    types: HashMap<TypeId, Vec<Handle>, BuildHasherDefault<TypeIdHasher>>,
    entities: Vec<Entity>,
}

impl World {
    pub fn new() -> Self {
        Self {
            free_handles: Vec::new(),
            types: HashMap::with_hasher(BuildHasherDefault::new()),
            entities: Vec::new(),
        }
    }

    /// returns `false` if `child` was already attached to `parent`.
    /// returns `err(invalidstructure)` if attaching would create an ownership cycle.
    /// detaches `child` from its current parent first (if it has one).
    /// registers `parent` in `self.types[child_ty]` if not already present.
    pub fn attach_child<T: EntityType>(
        &mut self,
        parent: Handle,
        child: Handle,
    ) -> Result<bool, Error> {
        if parent == child {
            return Err(Error::InvalidStructure);
        }
        // validate both handles
        if self.entities[parent].handle != parent {
            return Err(Error::Dead);
        }
        if self.entities[child].handle != child {
            return Err(Error::Dead);
        }

        // already attached?
        if self.entities[child].parent == parent {
            return Ok(false);
        }

        // cycle detection: walk the parent chain from `parent` upward.
        // if we reach `child` before reaching a root, attaching would form a loop.
        {
            let mut cur = parent;
            loop {
                let p = self.entities[cur].parent;
                if p == child {
                    return Err(Error::InvalidStructure);
                }
                if p == cur {
                    break; // self-parented root reached
                }
                cur = p;
            }
        }

        // detach child from its current parent (if not already a root)
        let old_parent = self.entities[child].parent;
        if old_parent != child {
            let child_ty = self.entities[child].ty;
            self.remove_child(old_parent, child, child_ty)?;
        }

        // entity.ty is the canonical key; t should match it.
        // the debug_assert catches caller mistakes in debug builds without
        // paying the price in release.
        let child_ty = self.entities[child].ty;
        debug_assert_eq!(child_ty, TypeId::of::<T>());

        let new_pos = self.entities[parent].children.len();
        self.entities[parent].children.push(child);
        self.entities[parent]
            .type_map
            .entry(child_ty)
            .or_default()
            .push(new_pos);

        self.entities[child].parent = parent;

        // register parent as owning a child of this type (idempotent)
        let parents_for_type = self.types.entry(child_ty).or_default();
        if !parents_for_type.contains(&parent) {
            parents_for_type.push(parent);
        }

        Ok(true)
    }

    /// removes `child` from `parent`'s children list and updates all bookkeeping.
    /// uses `swap_remove` for o(1) removal and patches up the displaced element's
    /// `type_map` index.
    /// removes `parent` from `self.types[child_ty]` when it has no remaining children
    /// of that type.
    pub fn remove_child(
        &mut self,
        parent: Handle,
        child: Handle,
        child_ty: TypeId,
    ) -> Result<(), Error> {
        if self.entities[parent].handle != parent {
            return Err(Error::Dead);
        }
        if self.entities[child].handle != child {
            return Err(Error::Dead);
        }

        let parent_idx = parent.index();

        // find which slot in `children` holds this child
        let child_pos = self.entities[parent_idx]
            .children
            .iter()
            .position(|&h| h == child)
            .ok_or(Error::Missing)?;

        // remove child_pos from the type_map entry for this child's type
        {
            let type_indices = self.entities[parent_idx]
                .type_map
                .get_mut(&child_ty)
                .ok_or(Error::Missing)?;
            let type_pos = type_indices
                .iter()
                .position(|&i| i == child_pos)
                .ok_or(Error::Missing)?;
            type_indices.swap_remove(type_pos);
        }

        // swap_remove the child from `children`. if a different element was moved
        // from `last_pos` into `child_pos`, patch its type_map index accordingly.
        let last_pos = self.entities[parent_idx].children.len() - 1;
        self.entities[parent_idx].children.swap_remove(child_pos);

        if child_pos != last_pos {
            // the element that was at `last_pos` is now at `child_pos`
            let displaced = self.entities[parent_idx].children[child_pos];
            let displaced_ty = self.entities[displaced].ty;
            if let Some(indices) = self.entities[parent_idx].type_map.get_mut(&displaced_ty) {
                if let Some(slot) = indices.iter().position(|&i| i == last_pos) {
                    indices[slot] = child_pos;
                }
            }
        }

        // if parent now has no children of child_ty, clean up self.types
        let now_empty = self.entities[parent_idx]
            .type_map
            .get(&child_ty)
            .map_or(true, |v| v.is_empty());
        if now_empty {
            self.entities[parent_idx].type_map.remove(&child_ty);
            if let Some(parents) = self.types.get_mut(&child_ty) {
                if let Some(pos) = parents.iter().position(|&h| h == parent) {
                    parents.swap_remove(pos);
                }
            }
        }

        // child becomes a root (self-parented)
        self.entities[child].parent = child;

        Ok(())
    }

    pub fn spawn<T: EntityType>(&mut self, value: T) -> Handle {
        let mut extend = false;
        let handle = self.free_handles.pop().unwrap_or_else(|| {
            extend = true;
            Handle::new(self.entities.len(), 0)
        });

        let ptr = Box::into_raw(Box::new(value)) as *mut ();

        let component = Entity {
            parent: handle,
            handle,
            ptr,
            drop_fn: T::drop_fn,
            ty: TypeId::of::<T>(),
            children: vec![],
            type_map: HashMap::with_hasher(BuildHasherDefault::new()),
        };
        if extend {
            self.entities.push(component);
        } else {
            self.entities[handle] = component;
        }
        handle
    }
    pub fn spawn_marker<T: EntityType>(&mut self, value: T::DowncastType) -> Handle {
        let mut extend = false;
        let handle = self.free_handles.pop().unwrap_or_else(|| {
            extend = true;
            Handle::new(self.entities.len(), 0)
        });

        let ptr = Box::into_raw(Box::new(value)) as *mut ();

        let component = Entity {
            parent: handle,
            handle,
            ptr,
            drop_fn: T::drop_fn,
            ty: TypeId::of::<T>(),
            children: vec![],
            type_map: HashMap::with_hasher(BuildHasherDefault::new()),
        };
        if extend {
            self.entities.push(component);
        } else {
            self.entities[handle] = component;
        }
        handle
    }

    /// detaches `handle` from its parent, unregisters it from `self.types`,
    /// recursively kills all children, calls `drop_fn`, then retires the slot.
    pub fn kill(&mut self, handle: Handle) -> Result<(), Error> {
        if self.entities[handle].handle != handle {
            return Err(Error::Dead);
        }

        // detach from parent if not a root
        let parent = self.entities[handle].parent;
        if parent != handle {
            let ty = self.entities[handle].ty;
            self.remove_child(parent, handle, ty)?;
        }

        // remove this entity from self.types (as a parent).
        // mem::take moves the map out without cloning, leaving an empty map in place.
        // the entity is dying so we don't need the map back.
        let type_map = std::mem::take(&mut self.entities[handle].type_map);
        for ty in type_map.keys() {
            if let Some(parents) = self.types.get_mut(ty) {
                if let Some(pos) = parents.iter().position(|&h| h == handle) {
                    parents.swap_remove(pos);
                }
            }
        }
        // type_map is dropped here; its heap allocation is freed

        // recursively kill all children.
        // mem::take avoids cloning and is safe: the entity is being torn down.
        // we set each child to self-parented first so the recursive kill
        // does not attempt remove_child on `handle` (which we're mid-destroying).
        let children = std::mem::take(&mut self.entities[handle].children);
        for child_handle in children {
            if self.entities[child_handle].handle == child_handle {
                self.entities[child_handle].parent = child_handle;
                self.kill(child_handle)?;
            }
        }
        // children vec is dropped here; its heap allocation is freed

        // drop the stored value via its registered drop function
        let ptr = self.entities[handle].ptr;
        let drop_fn = self.entities[handle].drop_fn;
        (drop_fn)(ptr);
        // null the pointer so a hypothetical double-kill crashes obviously rather than ubs
        self.entities[handle].ptr = std::ptr::null_mut();

        // retire the slot: bump the generation and add to free list.
        // any existing handles pointing to this index now have a stale generation
        // and will correctly fail the liveness check in entity()/entity_mut().
        let mut retired = handle;
        retired.increment();
        self.entities[handle].handle = retired;
        self.free_handles.push(retired);

        Ok(())
    }

    pub fn entity(&self, handle: Handle) -> Result<&Entity, Error> {
        let entry = &self.entities[handle];
        if entry.handle != handle {
            return Err(Error::Dead);
        }
        Ok(entry)
    }
    pub fn entity_mut(&mut self, handle: Handle) -> Result<&mut Entity, Error> {
        let entry = &mut self.entities[handle];
        if entry.handle != handle {
            return Err(Error::Dead);
        }
        Ok(entry)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Error {
    Dead,
    WrongType(TypeId),
    InvalidPtr,
    Missing,
    InvalidStructure,
}

#[derive(Default)]
pub struct TypeIdHasher {
    hash: u64,
}

impl Hasher for TypeIdHasher {
    fn write_u64(&mut self, n: u64) {
        debug_assert_eq!(self.hash, 0);
        self.hash = n;
    }
    fn write_u128(&mut self, n: u128) {
        debug_assert_eq!(self.hash, 0);
        self.hash = (n as u64) ^ ((n >> 64) as u64);
    }
    fn write(&mut self, bytes: &[u8]) {
        debug_assert_eq!(self.hash, 0);
        let mut hash = 0u64;
        for &b in bytes {
            hash = hash.wrapping_mul(131).wrapping_add(b as u64);
        }
        self.hash = hash;
    }
    fn finish(&self) -> u64 {
        self.hash
    }
}

pub trait EntityType
where
    Self: Sized + 'static,
{
    type DowncastType = Self;
    #[allow(unused)]
    fn kill_fn(this: Entity) {}
    fn drop_fn(this: *mut ()) {
        drop(unsafe { Box::from_raw(this as *mut Self::DowncastType) });
    }
}
