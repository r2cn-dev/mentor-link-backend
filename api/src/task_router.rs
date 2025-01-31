use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use chrono::{Datelike, Utc};
use common::{errors::CommonError, model::CommonResult};
use entity::{score, sea_orm_active_enums::TaskStatus, task};
use sea_orm::Set;
use service::storage::score_stg::ScoreRes;

use crate::{
    model::{
        score::NewScore,
        task::{CommandRequest, NewTask, SearchTask, Task},
    },
    AppState,
};

pub fn routers() -> Router<AppState> {
    Router::new().nest(
        "/task",
        Router::new()
            .route("/new", post(new_task))
            .route("/issue/{:github_issue_id}", get(get_task))
            .route("/search", post(search_with_status))
            .route("/request-assign", post(request_assign))
            .route("/intern-approve", post(intern_approve))
            .route("/release", post(release_task))
            .route("/request-complete", post(request_complete))
            .route("/intern-done", post(intern_done))
            .route("/intern-close", post(intern_close)),
    )
}

async fn new_task(
    state: State<AppState>,
    Json(json): Json<NewTask>,
) -> Result<Json<CommonResult<Task>>, CommonError> {
    let active_model = json.into();
    let res = state.task_stg().new_task(active_model).await.unwrap();
    Ok(Json(CommonResult::success(Some(res.into()))))
}

async fn get_task(
    state: State<AppState>,
    Path(github_issue_id): Path<i64>,
) -> Result<Json<CommonResult<Task>>, CommonError> {
    let res = state
        .task_stg()
        .search_task_with_issue_id(github_issue_id)
        .await
        .unwrap();

    let res: CommonResult<Task> = match res {
        Some(model) => CommonResult::success(Some(model.into())),
        None => CommonResult::failed("Task Not Found"),
    };
    Ok(Json(res))
}

async fn search_with_status(
    state: State<AppState>,
    Json(json): Json<SearchTask>,
) -> Result<Json<CommonResult<Vec<Task>>>, CommonError> {
    let res = state
        .task_stg()
        .search_task_with_status(
            json.github_repo_id,
            json.github_mentor_login,
            TaskStatus::processing_task_status(),
        )
        .await;
    let res = match res {
        Ok(model) => {
            let data = model.into_iter().map(|model| model.into()).collect();
            CommonResult::success(Some(data))
        }
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn request_assign(
    state: State<AppState>,
    Json(json): Json<CommandRequest>,
) -> Result<Json<CommonResult<bool>>, CommonError> {
    let res = state
        .task_stg()
        .request_assign(json.github_issue_id, json.login, json.student_name.unwrap())
        .await;

    let res = match res {
        Ok(_) => CommonResult::success(Some(true)),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn intern_approve(
    state: State<AppState>,
    Json(json): Json<CommandRequest>,
) -> Result<Json<CommonResult<bool>>, CommonError> {
    let res = state.task_stg().intern_approve(json.github_issue_id).await;

    let res = match res {
        Ok(_) => CommonResult::success(Some(true)),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn release_task(
    state: State<AppState>,
    Json(json): Json<CommandRequest>,
) -> Result<Json<CommonResult<bool>>, CommonError> {
    let res = state.task_stg().release_task(json.github_issue_id).await;

    let res = match res {
        Ok(_) => CommonResult::success(Some(true)),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn request_complete(
    state: State<AppState>,
    Json(json): Json<CommandRequest>,
) -> Result<Json<CommonResult<bool>>, CommonError> {
    let res = state
        .task_stg()
        .request_complete(json.github_issue_id)
        .await;

    let res = match res {
        Ok(_) => CommonResult::success(Some(true)),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn intern_done(
    state: State<AppState>,
    Json(json): Json<CommandRequest>,
) -> Result<Json<CommonResult<task::Model>>, CommonError> {
    let task = state
        .task_stg()
        .intern_done(json.github_issue_id)
        .await
        .unwrap();
    let score_stg = state.score_stg();
    let date = Utc::now();
    let current_score = score_stg
        .get_score(date.year(), date.month() as i32, json.login.clone())
        .await
        .unwrap();
    if let Some( score) = current_score {
        let sum_score = score.new_score + task.score;
        let mut a_model:score::ActiveModel = score.clone().into();
        a_model.new_score = Set(sum_score);
        score_stg.update_score(a_model).await.unwrap();
    } else {
        let last_score = score_stg
            .get_latest_score_by_login(json.login.clone())
            .await
            .unwrap();
        let mut new_score = NewScore {
            score: task.score,
            github_id: json.github_id,
            github_login: task.student_github_login.clone().unwrap(),
            student_name: task.student_name.clone().unwrap(),
            carryover_score: 0,
        };
        if let Some(last_score) = last_score {
            let score: ScoreRes = last_score.into();
            new_score.carryover_score = score.score_balance();
        }
        score_stg.insert_score(new_score.into()).await.unwrap();
    };
    Ok(Json(CommonResult::success(Some(task))))
}

async fn intern_close(
    state: State<AppState>,
    Json(json): Json<CommandRequest>,
) -> Result<Json<CommonResult<bool>>, CommonError> {
    state
        .task_stg()
        .intern_close(json.github_issue_id)
        .await
        .unwrap();
    Ok(Json(CommonResult::success(Some(true))))
}
