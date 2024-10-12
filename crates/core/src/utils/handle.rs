


/// [handle_option] 通用的msg处理方式实现error throw 的代码优雅程度。
///
/// # Example
///
/// ```no_run
///
/// struct LOC00007;
/// fn main()->Result<(),()>{
///     use edgelink_core::handle_option;
///     handle_option!(result: Result::Ok(""), etype: LOC00007, "x".clone() ,"y".clone());
/// }
///
/// ```
///

#[macro_export]
macro_rules! handle_option {
    (result: $result:expr,  $etype:expr ,  str:$arg:tt)=> {
       match $result {
           Some(re) => {re}
           None => {
                use crate::EdgelinkError;
                return Err($etype($arg.to_string()).into());
           }
       }
    };
}
#[macro_export]
macro_rules! get_json_value {
    ( $result:expr,  $key:tt, str)=> {
       $result.get($key).and_then(|x| x.as_str()).unwrap_or("")
    };
    ( $result:expr,  $key:tt, string)=> {
       $result.get($key).and_then(|x| x.to_string()).unwrap_or("".to_owned())
    };
}