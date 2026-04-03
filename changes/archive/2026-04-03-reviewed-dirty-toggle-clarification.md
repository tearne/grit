# Reviewed-Dirty Toggle Clarification

## Intent

The SPEC does not describe how toggling an unreviewed file interacts with the dirty overlay. During the `refresh` build it was found that toggling `Unreviewed → ReviewedDirty` immediately is confusing — the user just marked something reviewed and is told it is suspect before anything has changed. The correct behaviour is that toggling always produces `ReviewedStable`; the dirty overlay is applied separately by `merge_state` at load and refresh time. The SPEC should make this explicit so the distinction is clear to future readers.

## Approach

A single behaviour bullet is added to the SPEC under the existing `reviewed, dirty` behaviour rule (line 144). It states that the dirty overlay is applied by `merge_state` at load and refresh time only — toggling an unreviewed file always produces `ReviewedStable`, regardless of dirty status. No code changes are required; the implementation already behaves correctly after the `refresh` build.

## Plan

- [x] UPDATE SPEC: add a behaviour bullet after line 144 stating that toggling always produces `ReviewedStable`; the dirty overlay is applied by `merge_state` at load and refresh time only

## Conclusion

Delivered. The line number in the plan was stale; the bullet was added after the `reviewed, dirty` state definition in the File Checklist section.
