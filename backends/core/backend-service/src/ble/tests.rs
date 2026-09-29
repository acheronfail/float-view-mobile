use super::*;
fn transport(ceiling: usize, response: bool) -> Transport {
    let mut t = Transport::default();
    t.begin(7);
    t.profile(
        7,
        Profile {
            service: true,
            notify: true,
            with_response: if response { ceiling } else { 0 },
            without_response: ceiling,
        },
        8,
    )
    .unwrap();
    assert!(t.notifications(7, 8, true).unwrap());
    t
}
#[test]
fn filtering_and_profile_are_independent() {
    assert!(eligible(false, "saved", &["saved".into()]));
    assert!(!eligible(false, "Saved", &["saved".into()]));
    assert!(eligible(true, "guest", &[]));
    for service in [false, true] {
        for notify in [false, true] {
            let p = Profile {
                service,
                notify,
                with_response: 244,
                without_response: 20,
            };
            assert_eq!(p.select().is_some(), service && notify);
        }
    }
    for ceiling in [0, 513, usize::MAX] {
        assert!(
            Profile {
                service: true,
                notify: true,
                with_response: ceiling,
                without_response: 0
            }
            .select()
            .is_none()
        );
    }
    assert_eq!(
        Profile {
            service: true,
            notify: true,
            with_response: 512,
            without_response: 20
        }
        .select(),
        Some((WriteMode::WithResponse, 244))
    );
}
#[test]
fn atomic_admission_includes_current_chunk() {
    for ceiling in [1, 20, 244] {
        let mut t = transport(ceiling, true);
        let frame = vec![17; ceiling * 128];
        t.admit(&frame).unwrap();
        assert_eq!(t.next(9).unwrap().1.len(), ceiling);
        assert!(t.admit(&[18]).is_err());
        assert_eq!(t.bytes, frame.len());
        assert_eq!(t.queue.len(), 128);
        assert!(t.next(10).is_none());
        let mut empty = transport(ceiling, true);
        assert!(empty.admit(&vec![0; ceiling * 128 + 1]).is_err());
        assert!(empty.next(9).is_none());
    }
    assert!(fits(127, 65535, 1, 1));
    assert!(!fits(127, 65535, 1, 2));
    assert!(!fits(128, 0, 1, 0));
    assert!(!fits(usize::MAX, 0, 1, 0));
    assert!(!fits(0, 0, usize::MAX, usize::MAX));
}
#[test]
fn submission_and_completion_are_distinct_and_order_independent() {
    for response in [false, true] {
        for completion_first in [false, true] {
            let mut t = transport(20, response);
            let expected: Vec<u8> = (0..57).collect();
            t.admit(&expected[..25]).unwrap();
            t.admit(&expected[25..]).unwrap();
            let completion = if response {
                Progress::Acknowledged
            } else {
                Progress::Available
            };
            let unrelated = if response {
                Progress::Available
            } else {
                Progress::Acknowledged
            };
            let mut actual = vec![];
            for op in 9..13 {
                let (_, bytes) = t.next(op).unwrap();
                actual.extend(bytes);
                assert!(!t.progress(6, op, Progress::Submitted).unwrap());
                assert!(!t.progress(7, op + 1, completion).unwrap());
                assert!(!t.progress(7, op, unrelated).unwrap());
                let first = if completion_first {
                    completion
                } else {
                    Progress::Submitted
                };
                let last = if completion_first {
                    Progress::Submitted
                } else {
                    completion
                };
                assert!(!t.progress(7, op, first).unwrap());
                assert!(!t.progress(7, op, first).unwrap());
                assert!(t.next(99).is_none());
                assert!(t.progress(7, op, last).unwrap());
                assert!(!t.progress(7, op, last).unwrap());
            }
            assert_eq!(actual, expected);
            assert_eq!(t.bytes, 0);
        }
    }
}
#[test]
fn setup_and_replacement_reject_old_operations() {
    let mut t = transport(20, true);
    t.admit(&[1, 2, 3]).unwrap();
    t.next(9).unwrap();
    assert!(t.progress(7, 9, Progress::Failed).is_err());
    t.clear();
    assert!(!t.progress(7, 9, Progress::Submitted).unwrap());
    t.begin(10);
    assert!(!t.notifications(7, 8, true).unwrap());
    assert!(t.admit(&[1]).is_err());
    assert!(
        t.profile(
            7,
            Profile {
                service: false,
                notify: false,
                with_response: 0,
                without_response: 0
            },
            8
        )
        .unwrap()
        .is_none()
    );
}
