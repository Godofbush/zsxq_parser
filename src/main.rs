use std::fs::File;
use std::io::Write;

use mongodb::{Database, Collection};
use mongodb::{Client, options::ClientOptions};
use mongodb::bson::{doc, Document, RawDocument};
use percent_encoding::percent_decode_str;
use regex::Regex;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    // MongoDB 连接字符串
    #[arg(short, long, default_value_t = String::from("mongodb://127.0.0.1:27017"))]
    mongodb_uri: String,

    // Group id
    #[arg(short, long)]
    group_id: String,
}

struct Config<'a> {
    mongodb_uri: &'a str,
    group_id: &'a str,
}
impl<'a> Config<'a> {
    fn new(mongodb_uri: &'a str, group_id: &'a str) -> Self {
        Config {
            mongodb_uri,
            group_id
        }
    }

    fn get_topic_collection(&self) -> String {
        "topics_".to_string() + self.group_id
    }

    fn get_files_collection(&self) -> String {
        "files_".to_string() + self.group_id
    }

    fn get_images_collection(&self) -> String {
        "images_".to_string() + self.group_id
    }
}


#[tokio::main]
async fn main() {
    let args = Args::parse();

    println!("开始执行");
    let config = Config::new(&args.mongodb_uri, &args.group_id);
    println!("正在连接 MongoDB...");
    let db = db_conn(&config).await;
    println!("MongoDB 连接成功");

    let topic_coll = db.collection::<Document>(&config.get_topic_collection());
    let image_coll = db.collection::<Document>(&config.get_images_collection());
    let file_coll = db.collection::<Document>(&config.get_files_collection());

    let mut cursor = topic_coll.find(None, None).await.unwrap();
    let topic_count = topic_coll.count_documents(None, None).await.unwrap();

    let file_path = "./zsxq.md";
    let mut file = File::create(file_path).unwrap();

    file.write_all("".as_bytes()).unwrap();
    file.flush().unwrap();

    let mut handled_count = 0;
    print!("[{}/{}]正在执行中...", handled_count, topic_count);
    while cursor.advance().await.unwrap() {
        let raw_document = cursor.current();
        let raw_data = raw_document.get_document("raw_data").unwrap();
        if raw_data.get_str("type").unwrap() == "talk" {
            parse_talk(&image_coll, &file_coll, &mut file, raw_data).await;
        } else {
            // solution 类型不处理
        }
        handled_count += 1;
        print!("\r[{}/{}]正在执行中...", handled_count, topic_count);
    }
    println!("\r\n执行完毕");
}

/**
 * 建立数据连接
 */
async fn db_conn<'a>(config: &Config<'a>) -> Database {
    // 设置客户端选项
    let client_options = ClientOptions::parse(config.mongodb_uri).await.unwrap();

    // 连接到MongoDB
    let client = Client::with_options(client_options).unwrap();

    // 获取数据库
    client.database("zsxq")
}

/**
 * 解析话题数据
 */
async fn parse_talk(image_coll: &Collection<Document>, file_coll: &Collection<Document>, md: &mut File, raw_data: &RawDocument) {

    let topic_id = raw_data.get_i64("topic_id").unwrap();
    let create_time = raw_data.get_str("create_time").unwrap();
    let talk = raw_data.get_document("talk").unwrap();

    let topic_title = format!("## 话题 ID: {topic_id} - {create_time}\n\n");
    md.write(topic_title.as_bytes()).unwrap();


    let text_ret = talk.get_str("text");
    let article_ret = talk.get_document("article");

    if article_ret.is_ok() {
        // 文章话题
        if let Ok(text) = text_ret {
            let text = parse_content(text);
            let text = format!("{}", text);
            md.write(text.as_bytes()).unwrap();
        }
        if let Ok(article) = article_ret {
            let article_url = article.get_str("article_url").unwrap_or_else(|_| -> &str { "" });
            let article = format!("[阅读更多]({})\n\n", article_url);
            md.write(article.as_bytes()).unwrap();
        }
    } else {
        // 普通话题
        if let Ok(text) = text_ret {
            let text = parse_content(text);
            let text = format!("{} \n\n", text);
            md.write(text.as_bytes()).unwrap();
        }
    }

    if let Ok(images) =  talk.get_array("images") {
        for image in images {
            if let Ok(image) = image {
                if let Some(image) = image.as_document() {
                    let image_id = image.get_i64("image_id").unwrap();
                    let filter = doc! { "image_id": image_id, "type": "original" };
                    let image = image_coll.find_one(filter, None).await.unwrap();
                    if let Some(image) = image {
                        let image_url = image.get_str("target_dir").unwrap();
                        let image_text = format!("![{}]({})\n\n", image_id, image_url);
                        md.write(image_text.as_bytes()).unwrap();
                    }
                }
            }
        }
    }
    
    if let Ok(files) =  talk.get_array("files") {
        for file in files {
            if let Ok(file) = file {
                if let Some(file) = file.as_document() {
                    let file_id = file.get_i64("file_id").unwrap();
                    let file_name = file.get_str("name").unwrap();
                    let filter = doc! { "file_id": file_id };
                    let file = file_coll.find_one(filter, None).await.unwrap();
                    if let Some(file) = file {
                        let file_url = file.get_str("target_dir").unwrap();
                        let file_text = format!("![{}]({})\n\n", file_name, file_url);
                        md.write(file_text.as_bytes()).unwrap();
                    }
                }
            }
        }
    }

    md.write("\n".as_bytes()).unwrap();
}

/**
 * 解析话题内容
 */
fn parse_content(text: &str) -> String {
    // 创建正则表达式来匹配 <e> 标签的内容
    let link_re = Regex::new(r#"<e type="([^"]+)" href="([^"]+)" title="([^"]+)"[^/]*/>"#).unwrap();
    let tag_re = Regex::new(r#"<e type="hashtag" hid="([^"]+)" title="([^"]+)" />"#).unwrap();

    // 使用正则表达式替换文本中的 <e type="web"> 标签为 Markdown 格式的超链接
    let text = link_re.replace_all(text, |caps: &regex::Captures| {
        if let (Some(e_type), Some(href), Some(title)) = (caps.get(1), caps.get(2), caps.get(3)) {
            let e_type_str = e_type.as_str();
            let href_str = href.as_str();
            let title_str = title.as_str();

            let href_str = percent_decode_str(href_str).decode_utf8().unwrap();
            let title_str = percent_decode_str(title_str).decode_utf8().unwrap();

            if e_type_str == "web" {
                // 生成 Markdown 格式的超链接
                return format!("[{}]({})", title_str, href_str);
            } else {
                println!("Unexpected e type: {}", e_type_str);
            }
        }
        caps.get(0).unwrap().as_str().to_string() // 如果没有匹配成功则返回原始字符串
    });

    let text = tag_re.replace_all(&text, |caps: &regex::Captures| {
        if let (Some(hid), Some(title)) = (caps.get(1), caps.get(2)) {
            let _hid_str = hid.as_str();
            let title_str = title.as_str();

            let title_str = percent_decode_str(title_str).decode_utf8().unwrap();

            return format!("{}", title_str);
        }
        caps.get(0).unwrap().as_str().to_string() // 如果没有匹配成功则返回原始字符串
    });

    format!("{text}")
}
