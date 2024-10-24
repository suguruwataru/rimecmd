#include <rime_api.h>
#include <stdlib.h>
#include <string.h>

RimeApi *c_get_rime_api() { return rime_get_api(); }

// setup cannot be run a second time per process.
// The reason is, it initializes Google's log lib, glog, without
// checking whether it has already been initialized. The glog
// lib does not allow its initialization to be done twice per process.
void c_setup_rime_api_once(RimeApi *rime_api, const char *user_data_dir,
                           const char *shared_data_dir, int log_level) {
  RIME_STRUCT(RimeTraits, rime_traits);
  rime_traits.min_log_level = log_level;
  rime_traits.app_name = "rime.rimecmd";
  rime_traits.user_data_dir = user_data_dir;
  rime_traits.shared_data_dir = shared_data_dir;
  rime_api->setup(&rime_traits);
}

void c_initialize_rime_api(RimeApi *rime_api, const char *user_data_dir,
                           const char *shared_data_dir, int log_level) {
  RIME_STRUCT(RimeTraits, rime_traits);
  rime_traits.min_log_level = log_level;
  rime_traits.app_name = "rime.rimecmd";
  rime_traits.user_data_dir = user_data_dir;
  rime_traits.shared_data_dir = shared_data_dir;
  rime_api->initialize(&rime_traits);
}

void c_do_maintenance(RimeApi *rime_api) {
  // start maintenance returns True when the checks on the fs it does
  // all passed, and it starts a new process to "perform maintenance",
  // where it applies fs changes to the user data home directory.
  if (rime_api->start_maintenance(True)) {
    rime_api->join_maintenance_thread();
  }
}

void c_destory_rime_api(RimeApi *rime_api) { rime_api->finalize(); }

const char *c_get_user_data_dir(RimeApi *rime_api) {
  return rime_api->get_user_data_dir();
}

const char *c_get_shared_data_dir(RimeApi *rime_api) {
  return rime_api->get_shared_data_dir();
}

Bool c_get_schema_list(RimeApi *rime_api, RimeSchemaList *schema_list) {
  return rime_api->get_schema_list(schema_list);
}

void c_free_schema_list(RimeApi *rime_api, RimeSchemaList *schema_list) {
  rime_api->free_schema_list(schema_list);
}

RimeSessionId c_create_session(RimeApi *rime_api) {
  return rime_api->create_session();
}

void c_destory_session(RimeApi *rime_api, RimeSessionId session_id) {
  rime_api->destroy_session(session_id);
}

typedef struct rimecmd_rime_status {
  char *schema_name;
  char *schema_id;
  Bool is_disabled;
  Bool is_composing;
  Bool is_ascii_mode;
  Bool is_full_shape;
  Bool is_simplified;
  Bool is_traditional;
  Bool is_ascii_punct;
} RimecmdRimeStatus;

void c_get_status(RimeApi *rime_api, RimeSessionId session_id,
                  RimecmdRimeStatus *rimecmd_status) {
  RIME_STRUCT(RimeStatus, rime_status);
  rime_api->get_status(session_id, &rime_status);
  rimecmd_status->schema_name = strdup(rime_status.schema_name);
  rimecmd_status->schema_id = strdup(rime_status.schema_id);
  rimecmd_status->is_disabled = rime_status.is_disabled;
  rimecmd_status->is_composing = rime_status.is_composing;
  rimecmd_status->is_ascii_mode = rime_status.is_ascii_mode;
  rimecmd_status->is_full_shape = rime_status.is_full_shape;
  rimecmd_status->is_simplified = rime_status.is_simplified;
  rimecmd_status->is_traditional = rime_status.is_traditional;
  rimecmd_status->is_ascii_punct = rime_status.is_ascii_punct;
  rime_api->free_status(&rime_status);
}

void c_free_status(RimecmdRimeStatus *status) {
  free(status->schema_name);
  free(status->schema_id);
}

typedef struct rimecmd_rime_commit {
  char *text;
} RimecmdRimeCommit;

void c_get_commit(RimeApi *rime_api, RimeSessionId session_id,
                  RimecmdRimeCommit *rimecmd_commit) {
  RIME_STRUCT(RimeCommit, rime_commit);
  rime_api->get_commit(session_id, &rime_commit);
  if (rime_commit.text) {
    rimecmd_commit->text = strdup(rime_commit.text);
  }
  rime_api->free_commit(&rime_commit);
}

void c_free_commit(RimecmdRimeCommit *commit) {
  if (commit->text)
    free(commit->text);
}

Bool c_process_key(RimeApi *rime_api, RimeSessionId session_id, int keycode,
                   int mask) {
  return rime_api->process_key(session_id, keycode, mask);
}

typedef struct rimecmd_rime_context {
  RimeComposition composition;
  RimeMenu menu;
  char *commit_text_preview;
} RimecmdRimeContext;

void c_get_context(RimeApi *rime_api, RimeSessionId session_id,
                   RimecmdRimeContext *rimecmd_context) {
  RIME_STRUCT(RimeContext, rime_context);
  rime_api->get_context(session_id, &rime_context);
  if (rime_context.commit_text_preview) {
    rimecmd_context->commit_text_preview =
        strdup(rime_context.commit_text_preview);
  }
  rimecmd_context->composition = rime_context.composition;
  if (rime_context.composition.preedit)
    rimecmd_context->composition.preedit =
        strdup(rime_context.composition.preedit);
  rimecmd_context->menu = rime_context.menu;
  rime_api->free_context(&rime_context);
}

void c_free_context(RimecmdRimeContext *context) {
  if (context->commit_text_preview)
    free(context->commit_text_preview);
  if (context->composition.preedit)
    free(context->composition.preedit);
}

Bool c_get_current_schema(RimeApi *rime_api, RimeSessionId session_id,
                          char *schema_id, size_t buffer_size) {
  return rime_api->get_current_schema(session_id, schema_id, buffer_size);
}

Bool c_candidate_list_begin(RimeApi *rime_api, RimeSessionId session_id,
                            RimeCandidateListIterator *iterator) {
  return rime_api->candidate_list_begin(session_id, iterator);
}

Bool c_candidate_list_next(RimeApi *rime_api,
                           RimeCandidateListIterator *iterator) {
  return rime_api->candidate_list_next(iterator);
}

void c_candidate_list_end(RimeApi *rime_api,
                          RimeCandidateListIterator *iterator) {
  rime_api->candidate_list_end(iterator);
}
