pub struct Validator<'a, T, C, U> {
    command: &'a C,
    validate_list: Vec<Box<dyn Validate<T, C, U>>>,
}

impl<'a, T, C, U> Validator<'a, T, C, U> {
    pub fn new(command: &'a C) -> Validator<'a, T, C, U> {
        Self {
            command,
            validate_list: Vec::new(),
        }
    }

    pub fn should(mut self, validator: impl Validate<T, C, U> + 'static) -> Self {
        self.validate_list.push(Box::new(validator));
        self
    }

    pub fn validate_against(self, domain: &T) -> Result<&'a C, (&'a C, Vec<U>)> {
        let mut errors = Vec::new();
        for validator in self.validate_list {
            if let Some(error) = validator.validate(domain, self.command) {
                errors.push(error);
            }
        }
        if errors.is_empty() {
            Ok(self.command)
        } else {
            Err((self.command, errors))
        }
    }
}

pub trait Validate<T, C, U> {
    fn validate(&self, domain: &T, command: &C) -> Option<U>;
}

impl<V, T, U, C> Validate<T, C, U> for V
where
    V: Fn(&T, &C) -> Option<U>,
{
    fn validate(&self, domain: &T, command: &C) -> Option<U> {
        self(domain, command)
    }
}

pub struct ChainedValidate<T, C, U> {
    validate_list: Vec<Box<dyn Validate<T, C, U>>>,
}

impl<T, C, U> ChainedValidate<T, C, U> {
    pub fn new(validate: impl Validate<T, C, U> + 'static) -> ChainedValidate<T, C, U> {
        ChainedValidate {
            validate_list: vec![Box::new(validate)],
        }
    }

    pub fn and(mut self, validate: impl Validate<T, C, U> + 'static) -> ChainedValidate<T, C, U> {
        self.validate_list.push(Box::new(validate));
        self
    }
}

pub trait ValidateCommand<'a> {
    fn should<T, U>(
        &'a self,
        validate: impl Validate<T, Self, U> + 'static,
    ) -> Validator<'a, T, Self, U>
    where
        Self: Sized;
}

impl<'a, C> ValidateCommand<'a> for C
where
    C: Sized,
{
    fn should<T, U>(
        &'a self,
        validate: impl Validate<T, C, U> + 'static,
    ) -> Validator<'a, T, Self, U> {
        Validator::<'a, T, C, U>::new(self).should(validate)
    }
}

impl<T, C, U> Validate<T, C, U> for ChainedValidate<T, C, U> {
    fn validate(&self, domain: &T, command: &C) -> Option<U> {
        self.validate_list
            .iter()
            .find_map(|validate| validate.validate(domain, command))
    }
}

pub trait ChainedValidateExt<T, C, U> {
    fn and(self, validate: impl Validate<T, C, U> + 'static) -> ChainedValidate<T, C, U>;
}

impl<F, T, C, U> ChainedValidateExt<T, C, U> for F
where
    F: Validate<T, C, U> + 'static,
{
    fn and(self, validate: impl Validate<T, C, U> + 'static) -> ChainedValidate<T, C, U> {
        ChainedValidate::new(self).and(validate)
    }
}
