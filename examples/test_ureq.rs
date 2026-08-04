fn main() {
    let url = "https://www.svg.com/img/gallery/its-clear-why-theburntpeanut-subs-have-skyrocketed-by-4482/intro-1767902355.webp";
    match ureq::get(url).call() {
        Ok(mut resp) => {
            println!("status: {}", resp.status());
            println!("mime: {:?}", resp.body_mut().mime_type());
            match resp.body_mut().read_to_vec() {
                Ok(v) => println!("bytes: {}", v.len()),
                Err(e) => println!("read err: {e}"),
            }
        }
        Err(e) => println!("call err: {e:?}"),
    }
}
