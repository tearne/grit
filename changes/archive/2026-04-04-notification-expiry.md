# Notification Expiry

## Intent

Copy confirmations currently persist until the user navigates to a different file. The stored timestamp is never checked, so the notification has no time limit. Notifications should expire automatically after a short interval so they do not linger and obscure the footer hint text.

## Approach

At the top of the checklist loop, before rendering, check whether the stored `Instant` in `notification` is older than 3 seconds and if so clear it. A named constant `NOTIFICATION_TTL: Duration` captures the interval. No other changes — the `Instant` is already stored; it just needs to be read.

Review cadence: at the end.

## Plan

- [x] ADD IMPL: `NOTIFICATION_TTL: Duration` constant in `tui.rs` set to 3 seconds
- [x] UPDATE IMPL: checklist loop in `tui.rs` — at the top of the loop, before rendering, clear `notification` if the stored `Instant` has elapsed beyond `NOTIFICATION_TTL`

## Conclusion

Delivered as planned. No surprises.
