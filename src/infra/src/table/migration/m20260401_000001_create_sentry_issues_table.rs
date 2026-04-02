use sea_orm_migration::prelude::*;

const SENTRY_ISSUES_ORG_FP_UNIQUE: &str = "sentry_issues_org_fingerprint_unique";
const SENTRY_ISSUES_ORG_STATUS_IDX: &str = "sentry_issues_org_status_idx";
const SENTRY_ISSUES_LAST_SEEN_IDX: &str = "sentry_issues_last_seen_idx";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(create_sentry_issues_table())
            .await?;
        manager
            .create_index(create_org_fingerprint_unique_idx())
            .await?;
        manager
            .create_index(create_org_status_idx())
            .await?;
        manager
            .create_index(create_last_seen_idx())
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name(SENTRY_ISSUES_LAST_SEEN_IDX)
                    .table(SentryIssues::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name(SENTRY_ISSUES_ORG_STATUS_IDX)
                    .table(SentryIssues::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name(SENTRY_ISSUES_ORG_FP_UNIQUE)
                    .table(SentryIssues::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(SentryIssues::Table).to_owned())
            .await
    }
}

fn create_sentry_issues_table() -> TableCreateStatement {
    Table::create()
        .table(SentryIssues::Table)
        .if_not_exists()
        .col(
            ColumnDef::new(SentryIssues::Id)
                .char_len(27)
                .not_null()
                .primary_key(),
        )
        .col(
            ColumnDef::new(SentryIssues::OrgId)
                .string_len(128)
                .not_null(),
        )
        // SHA256 hex fingerprint (64 chars)
        .col(
            ColumnDef::new(SentryIssues::Fingerprint)
                .char_len(64)
                .not_null(),
        )
        .col(
            ColumnDef::new(SentryIssues::Status)
                .string_len(20)
                .not_null()
                .default("open"),
        )
        .col(
            ColumnDef::new(SentryIssues::Title)
                .string_len(500)
                .not_null()
                .default(""),
        )
        .col(
            ColumnDef::new(SentryIssues::Culprit)
                .string_len(500)
                .not_null()
                .default(""),
        )
        .col(
            ColumnDef::new(SentryIssues::ExceptionType)
                .string_len(256)
                .not_null()
                .default(""),
        )
        .col(
            ColumnDef::new(SentryIssues::Platform)
                .string_len(64)
                .null(),
        )
        .col(
            ColumnDef::new(SentryIssues::Release)
                .string_len(256)
                .null(),
        )
        .col(
            ColumnDef::new(SentryIssues::Environment)
                .string_len(128)
                .null(),
        )
        .col(
            ColumnDef::new(SentryIssues::TimesSeen)
                .big_integer()
                .not_null()
                .default(1),
        )
        .col(
            ColumnDef::new(SentryIssues::UsersSeen)
                .big_integer()
                .not_null()
                .default(0),
        )
        .col(
            ColumnDef::new(SentryIssues::FirstSeen)
                .big_integer()
                .not_null(),
        )
        .col(
            ColumnDef::new(SentryIssues::LastSeen)
                .big_integer()
                .not_null(),
        )
        .col(
            ColumnDef::new(SentryIssues::LastEventId)
                .string_len(36)
                .not_null()
                .default(""),
        )
        .col(
            ColumnDef::new(SentryIssues::AssignedTo)
                .string_len(256)
                .null(),
        )
        .col(
            ColumnDef::new(SentryIssues::CreatedAt)
                .big_integer()
                .not_null(),
        )
        .col(
            ColumnDef::new(SentryIssues::UpdatedAt)
                .big_integer()
                .not_null(),
        )
        .to_owned()
}

fn create_org_fingerprint_unique_idx() -> IndexCreateStatement {
    Index::create()
        .if_not_exists()
        .name(SENTRY_ISSUES_ORG_FP_UNIQUE)
        .table(SentryIssues::Table)
        .col(SentryIssues::OrgId)
        .col(SentryIssues::Fingerprint)
        .unique()
        .to_owned()
}

fn create_org_status_idx() -> IndexCreateStatement {
    Index::create()
        .if_not_exists()
        .name(SENTRY_ISSUES_ORG_STATUS_IDX)
        .table(SentryIssues::Table)
        .col(SentryIssues::OrgId)
        .col(SentryIssues::Status)
        .to_owned()
}

fn create_last_seen_idx() -> IndexCreateStatement {
    Index::create()
        .if_not_exists()
        .name(SENTRY_ISSUES_LAST_SEEN_IDX)
        .table(SentryIssues::Table)
        .col(SentryIssues::LastSeen)
        .to_owned()
}

#[derive(DeriveIden)]
enum SentryIssues {
    Table,
    Id,
    OrgId,
    Fingerprint,
    Status,
    Title,
    Culprit,
    ExceptionType,
    Platform,
    Release,
    Environment,
    TimesSeen,
    UsersSeen,
    FirstSeen,
    LastSeen,
    LastEventId,
    AssignedTo,
    CreatedAt,
    UpdatedAt,
}
