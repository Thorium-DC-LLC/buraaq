use buraaq_ownership::Place;
use buraaq_source::Span;
pub use buraaq_types::Region;

/// Shared or exclusive borrow of a place.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LoanKind {
    Shared,
    Mut,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Loan {
    pub place: Place,
    pub kind: LoanKind,
    pub region: Region,
    pub span: Span,
}

#[derive(Debug, Default)]
pub struct LoanSet {
    loans: Vec<Loan>,
    next_region: u32,
}

impl LoanSet {
    pub fn new() -> Self {
        Self {
            loans: Vec::new(),
            next_region: 1,
        }
    }

    pub fn fresh_region(&mut self) -> Region {
        let r = Region(self.next_region);
        self.next_region += 1;
        r
    }

    pub fn active(&self) -> &[Loan] {
        &self.loans
    }

    pub fn start_loan(&mut self, place: Place, kind: LoanKind, region: Region, span: Span) {
        self.loans.push(Loan {
            place,
            kind,
            region,
            span,
        });
    }

    pub fn end_region(&mut self, region: Region) {
        self.loans.retain(|l| l.region != region);
    }

    pub fn conflicting(&self, place: Place, kind: LoanKind) -> Option<&Loan> {
        for loan in &self.loans {
            if loan.place.local != place.local {
                continue;
            }
            match (loan.kind, kind) {
                (LoanKind::Mut, _) | (_, LoanKind::Mut) => return Some(loan),
                (LoanKind::Shared, LoanKind::Shared) => {}
            }
        }
        None
    }
}
