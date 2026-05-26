


#[cfg(test)]
mod tests {
    use sparky_macros::sql;
    use super::*;

    #[test]
    fn compile_error() {
        sql![select alt a from c]
    }

    #[test]
    fn should_work_select() {

    }

    #[test]
    fn should_work_select_filter() {

    }

    #[test]
    fn should_work_delete() {

    }
}
