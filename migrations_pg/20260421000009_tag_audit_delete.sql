-- Allow direct concept deletion by any logged-in user (audit trail
-- captures every removal, so admin review is unnecessary friction).
-- Add 'delete_tag' to the audit-action whitelist.
ALTER TABLE tag_audit_log DROP CONSTRAINT tag_audit_log_action_check;
ALTER TABLE tag_audit_log ADD CONSTRAINT tag_audit_log_action_check
    CHECK (action IN ('create_tag', 'add_name', 'remove_name', 'merge_tag', 'delete_tag'));
