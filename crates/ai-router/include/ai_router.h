/* C ABI of the ai-router crate (shared Aphrody C ABI: JSON strings returned by these functions are freed with
 * aphrody_free). Requests and answers are OpenAI chat-completions documents. */
#ifndef AI_ROUTER_H
#define AI_ROUTER_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct AiRouter AiRouter;

uint32_t aphrody_abi_version(void);
void aphrody_free(char *p);

/* config_json: NULL, "" or {"catalog_url","catalog","local":[base URLs],"max_attempts","timeout_s"}.
 * Returns NULL on error; aphrody_router_last_error() then gives {"error": "..."}. */
AiRouter *aphrody_router_open(const char *config_json);
char *aphrody_router_last_error(void);

/* request_json: OpenAI chat-completions request; "model" is a policy (auto, auto:local, auto:free, auto:cheap) or
 * an offer id (<provider>/<model>). The answer carries "router": {"offer", "local", "zero_cost", "attempts"}. */
char *aphrody_router_chat(const AiRouter *router, const char *request_json);

/* Streams the answer: on_chunk receives each chat.completion.chunk (JSON valid during the call) on the calling thread;
 * offers are tried until one starts streaming. Returns {"router": {...}} or an error document. */
typedef void (*AiRouterChunk)(const char *chunk_json, void *user);
char *aphrody_router_chat_stream(const AiRouter *router, const char *request_json, AiRouterChunk on_chunk, void *user);

/* Usable offers for a policy or an offer id, in the order they are tried (JSON array). */
char *aphrody_router_models(const AiRouter *router, const char *model);

void aphrody_router_close(AiRouter *router);

#ifdef __cplusplus
}
#endif

#endif
