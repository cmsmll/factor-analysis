use std::fs;
use std::time::Instant;

use salvo::{cors::Cors, prelude::*};
use salvo_oapi::{Info, OpenApi, Tag, swagger_ui::SwaggerUi};

use crate::{CONFIG, logger::Logger, router};
/// 根据业务路由生成 OpenAPI 文档。
pub(crate) fn build_openapi(router: &Router) -> OpenApi {
    OpenApi::with_info(
        Info::new("Factor Analysis API", env!("CARGO_PKG_VERSION")).description("因子分析服务接口，包含基础数据查询、模式一因子模板和分位分析。"),
    )
    .tags([
        Tag::new("系统").description("服务状态接口。"),
        Tag::new("基础数据").description("股票池指数与行业板块数据。"),
        Tag::new("模式一").description("按因子值排序的分位分析接口。"),
        Tag::new("测试").description("固定参数测试接口。"),
    ])
    .merge_router(router)
}

/// Web 服务运行命令。
#[derive(Debug, clap::Args)]
pub struct RunCommand {
    /// 启动前清空缓存
    #[arg(short = 'c', long)]
    pub clear: bool,
}

impl RunCommand {
    pub(super) async fn execute(self) {
        if self.clear {
            println!("正在清空缓存...");
            if CONFIG.data.cache.exists() && let Err(err) = fs::remove_dir_all(&CONFIG.data.cache) {
                eprintln!("清空缓存失败: {err}");
            }
        }

        let now = Instant::now();
        println!("正在加载数据库...");
        let router = router::router().await;
        let openapi = build_openapi(&router);

        let router = router.unshift(openapi.into_router("/api-doc/openapi.json")).unshift(
            SwaggerUi::new("/api-doc/openapi.json")
                .title("Factor Analysis API")
                .into_router("/swagger-ui"),
        );
        let addr = CONFIG.socket_addr();

        println!("数据库加载完成，耗时: {:.2?}", now.elapsed());
        println!("WebService running at: http://{addr}");
        println!("Swagger UI: http://{addr}/swagger-ui");
        println!("OpenAPI JSON: http://{addr}/api-doc/openapi.json");

        let cors = Cors::permissive().into_handler();
        let acceptor = TcpListener::new(addr).bind().await;
        let server = Service::new(router).hoop(cors).hoop(Logger::default());
        Server::new(acceptor).serve(server).await;
    }
}
