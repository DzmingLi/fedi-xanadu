-- Book-deletion requests: any signed-in user can request a book be deleted;
-- an admin must approve. Approving sets books.removed_at (soft delete);
-- frontend/API hides books where removed_at IS NOT NULL.

ALTER TABLE books ADD COLUMN IF NOT EXISTS removed_at TIMESTAMPTZ;

CREATE TABLE book_deletion_requests (
    id            VARCHAR(64) PRIMARY KEY DEFAULT 'bdr-' || substr(md5(random()::text), 1, 12),
    book_id       VARCHAR(64) NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    requester_did VARCHAR(255) NOT NULL,
    reason        TEXT        NOT NULL,
    status        VARCHAR(20) NOT NULL DEFAULT 'pending'
                   CHECK (status IN ('pending','approved','rejected','cancelled')),
    reviewer_did  VARCHAR(255),
    review_note   TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    reviewed_at   TIMESTAMPTZ
);

CREATE INDEX idx_book_deletion_requests_book    ON book_deletion_requests(book_id);
CREATE INDEX idx_book_deletion_requests_status  ON book_deletion_requests(status);

-- A user can only have one *pending* request per book.
CREATE UNIQUE INDEX idx_book_deletion_requests_one_pending
    ON book_deletion_requests(book_id, requester_did) WHERE status = 'pending';
