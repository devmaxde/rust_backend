pub(crate) enum ApiTags {
    Auth,
}

impl ApiTags {
    fn as_str(&self) -> &'static str {
        match self {
            ApiTags::Auth => "Auth",
        }
    }
}

impl From<ApiTags> for &'static str {
    fn from(tag: ApiTags) -> Self {
        tag.as_str()
    }
}

impl From<ApiTags> for String {
    fn from(tag: ApiTags) -> Self {
        tag.as_str().to_string()
    }
}
