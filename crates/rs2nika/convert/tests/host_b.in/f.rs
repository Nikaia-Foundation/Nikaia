    fn files(&self) -> (Vec<&Parsed>, usize) {
        let mut files: Vec<&Parsed> = self.beside.to_vec();
        let here = match self.here {
            Some(here) => here,
            None => {
                files.insert(0, self.parsed);
                0
            }
        };
        (files, here)
    }
}

/// What a grammar's wall is, as the report of it is given it.
pub(super) fn a_wall(why: &crate::grammar_run::Wall) -> CheckWall {
    use crate::grammar_run::Wall;
    match why {
        Wall::Refused { detail } => CheckWall::Refused {
            detail: detail.clone(),
        },
        Wall::NoCrossedForm { ty, because } => CheckWall::NoCrossedForm {
            ty: ty.clone(),
            because: because.clone(),
        },
        Wall::InsideAnother { grammar } => CheckWall::InsideAnother {
            outer: grammar.clone(),
        },
        Wall::NowhereToBuild => CheckWall::NowhereToBuild,
        Wall::DidNotBuild { detail } => CheckWall::DidNotBuild {
            detail: detail.clone(),
        },
        Wall::Unreadable { detail } => CheckWall::Unreadable {
            detail: detail.clone(),
        },
    }
}

impl CheckHost for Host<'_> {
