# Target verification API docs source gate

Before patch, reread product target_verification.rs constructor/validation and
its tests, and OpenDev state_snapshot persistence source/tests at revision
d32c660e4eed1a8e988d1fd58da88e41ba641d08 (MIT). Adopt explicit documentation
of validation errors and keep authentication boundary explicit; avoid claiming
the domain observation authenticates its producer. No upstream code copied.

Planned patch adds Errors and must_use only; existing behavior/tests remain the
authority. Foundation/fmt are required; full authenticated target acceptance is
outside this lint slice.
