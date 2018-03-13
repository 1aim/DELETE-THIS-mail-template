
///// like try on a result but converts the error to a boxed error-future before returning it
//#[cfg(feature="smtp")]
//macro_rules! r2f_try {
//    ($code:expr) => ({
//        use futures::future;
//        match $code {
//            Ok(val) => val,
//            Err(error) => return Box::new(future::err(error))
//        }
//    });
//}

///
/// ```
/// cloned!([service] => move |name| {
///     drop(service)
/// })
/// ```
#[cfg(feature="smtp")]
macro_rules! cloned {
    ([$($toclo:ident),*] => $doit:expr) => ({
        $(
            let $toclo = $toclo.clone();
        )*
        $doit
    });
}

/// like try_ready but for streams also checking it's some
#[cfg(features="smtp")]
macro_rules! try_some_ready {
    ($e:expr) => ({
        match $e {
            Ok(Async::Ready(Some(item))) => item,
            Ok(other) => return Ok(other),
            Err(err) => return Err(From::from(err))
        }
    });
}