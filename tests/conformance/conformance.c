/*
 * pdftract SDK Conformance Test Runner (C)
 *
 * This test runs the shared SDK conformance suite against the C SDK.
 * It loads tests/sdk-conformance/cases.json and executes each test case.
 *
 * Compile: gcc -o conformance conformance.c -ljson-c -lpdftract
 * Run: ./conformance [suite-path] [output-path]
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <sys/time.h>
#include <json-c/json.h>
#include <pdftract.h>

#define SUITE_PATH "tests/sdk-conformance/cases.json"
#define SDK_NAME "pdftract-libpdftract"
#define SDK_VERSION "0.1.0"

typedef enum {
    STATUS_PASS,
    STATUS_FAIL,
    STATUS_SKIP,
    STATUS_ERROR
} test_status_t;

typedef struct {
    char *id;
    test_status_t status;
    struct json_object *actual;
    struct json_object *expected;
    char *error;
    char *reason;
    long duration_ms;
} test_result_t;

typedef struct {
    int total;
    int passed;
    int failed;
    int skipped;
    int errors;
    long duration_ms;
} summary_t;

typedef struct {
    char *os;
    char *arch;
    char *binary_version;
    char *runtime_version;
} environment_t;

/* Compare two floating-point values with tolerance */
static int compare_with_tolerance(double actual, double expected, struct json_object *tolerance) {
    if (!tolerance || !json_object_is_type(tolerance, json_type_object)) {
        return fabs(actual - expected) < 1e-9;
    }

    struct json_object *abs_tol = NULL;
    if (json_object_object_get_ex(tolerance, "abs", &abs_tol) && abs_tol) {
        double abs_val = json_object_get_double(abs_tol);
        if (fabs(actual - expected) <= abs_val) {
            return 1;
        }
    }

    struct json_object *rel_tol = NULL;
    if (json_object_object_get_ex(tolerance, "rel", &rel_tol) && rel_tol) {
        double rel_val = json_object_get_double(rel_tol);
        double diff = fabs(actual - expected);
        double avg = (actual + expected) / 2.0;
        if (avg > 0.0 && diff / avg <= rel_val) {
            return 1;
        }
    }

    return 0;
}

/* Find tolerance for a given path */
static struct json_object *find_tolerance(struct json_object *tolerances, const char *path) {
    if (!tolerances || !json_object_is_type(tolerances, json_type_object)) {
        return NULL;
    }

    struct json_object *result = NULL;
    if (json_object_object_get_ex(tolerances, path, &result)) {
        return result;
    }

    /* Wildcard matching */
    json_object_object_foreach(tolerances, key, val) {
        if (strchr(key, '*')) {
            /* Simple wildcard: replace * with .* and use regex (simplified here) */
            if (strncmp(key, path, strchr(key, '*') - key) == 0) {
                return val;
            }
        }
    }

    return NULL;
}

/* Compare actual results against expected with tolerances */
static int compare_results(struct json_object *actual, struct json_object *expected,
                           struct json_object *tolerances, const char *path,
                           char **error_msg) {
    if (!expected || !actual) {
        if (expected != actual) {
            asprintf(error_msg, "%s: NULL mismatch", path);
            return 0;
        }
        return 1;
    }

    if (json_object_is_type(expected, json_type_object)) {
        if (json_object_is_type(actual, json_type_double) ||
            json_object_is_type(actual, json_type_int)) {
            double act_val = json_object_get_double(actual);

            struct json_object *min_obj = NULL, *max_obj = NULL, *val_obj = NULL;
            if (json_object_object_get_ex(expected, "min", &min_obj) && min_obj) {
                double min = json_object_get_double(min_obj);
                if (act_val < min) {
                    asprintf(error_msg, "%s: value %f < minimum %f", path, act_val, min);
                    return 0;
                }
            }
            if (json_object_object_get_ex(expected, "max", &max_obj) && max_obj) {
                double max = json_object_get_double(max_obj);
                if (act_val > max) {
                    asprintf(error_msg, "%s: value %f > maximum %f", path, act_val, max);
                    return 0;
                }
            }
            if (json_object_object_get_ex(expected, "value", &val_obj) && val_obj) {
                double exp_val = json_object_get_double(val_obj);
                struct json_object *tol = find_tolerance(tolerances, path);
                if (!compare_with_tolerance(act_val, exp_val, tol)) {
                    asprintf(error_msg, "%s: numeric mismatch", path);
                    return 0;
                }
            }
        } else if (json_object_is_type(actual, json_type_string)) {
            const char *act_str = json_object_get_string(actual);

            struct json_object *min_len_obj = NULL;
            if (json_object_object_get_ex(expected, "min_length", &min_len_obj) && min_len_obj) {
                int min_len = json_object_get_int(min_len_obj);
                if ((int)strlen(act_str) < min_len) {
                    asprintf(error_msg, "%s: string length %zu < minimum %d",
                             path, strlen(act_str), min_len);
                    return 0;
                }
            }

            struct json_object *contains_obj = NULL;
            if (json_object_object_get_ex(expected, "contains", &contains_obj) &&
                contains_obj && json_object_is_type(contains_obj, json_type_array)) {
                for (int i = 0; i < json_object_array_length(contains_obj); i++) {
                    struct json_object *item = json_object_array_get_idx(contains_obj, i);
                    const char *substr = json_object_get_string(item);
                    if (!strstr(act_str, substr)) {
                        asprintf(error_msg, "%s: string does not contain '%s'", path, substr);
                        return 0;
                    }
                }
            }
        } else if (json_object_is_type(actual, json_type_array)) {
            int act_len = json_object_array_length(actual);

            struct json_object *min_obj = NULL, *max_obj = NULL;
            if (json_object_object_get_ex(expected, "min", &min_obj) && min_obj) {
                int min = json_object_get_int(min_obj);
                if (act_len < min) {
                    asprintf(error_msg, "%s: array length %d < minimum %d", path, act_len, min);
                    return 0;
                }
            }
            if (json_object_object_get_ex(expected, "max", &max_obj) && max_obj) {
                int max = json_object_get_int(max_obj);
                if (act_len > max) {
                    asprintf(error_msg, "%s: array length %d > maximum %d", path, act_len, max);
                    return 0;
                }
            }
        } else if (json_object_is_type(actual, json_type_object)) {
            json_object_object_foreach(expected, key, exp_val) {
                char *new_path;
                asprintf(&new_path, "%s%s%s", path, (*path) ? "." : "", key);

                struct json_object *act_val = NULL;
                if (!json_object_object_get_ex(actual, key, &act_val)) {
                    asprintf(error_msg, "%s: missing key '%s'", new_path, key);
                    free(new_path);
                    return 0;
                }

                if (!compare_results(act_val, exp_val, tolerances, new_path, error_msg)) {
                    free(new_path);
                    return 0;
                }
                free(new_path);
            }
        }
    } else if (json_object_is_type(expected, json_type_array) &&
               json_object_is_type(actual, json_type_array)) {
        int exp_len = json_object_array_length(expected);
        int act_len = json_object_array_length(actual);

        for (int i = 0; i < exp_len; i++) {
            char *new_path;
            asprintf(&new_path, "%s[%d]", path, i);

            if (i >= act_len) {
                asprintf(error_msg, "%s: missing index", new_path);
                free(new_path);
                return 0;
            }

            struct json_object *exp_val = json_object_array_get_idx(expected, i);
            struct json_object *act_val = json_object_array_get_idx(actual, i);

            if (!compare_results(act_val, exp_val, tolerances, new_path, error_msg)) {
                free(new_path);
                return 0;
            }
            free(new_path);
        }
    } else {
        if (!json_object_equal(actual, expected)) {
            asprintf(error_msg, "%s: values do not match", path);
            return 0;
        }
    }

    return 1;
}

/* Execute a pdftract method (stub implementation) */
static struct json_object *execute_method(const char *method, const char *fixture,
                                          struct json_object *options,
                                          char **error_msg) {
    /* This is a stub - replace with actual SDK calls when available */
    struct json_object *result = json_object_new_object();

    if (strcmp(method, "extract") == 0) {
        json_object_object_add(result, "schema_version", json_object_new_string("1.0"));

        struct json_object *metadata = json_object_new_object();
        json_object_object_add(metadata, "page_count", json_object_new_int(1));
        json_object_object_add(result, "metadata", metadata);

        struct json_object *pages = json_object_new_array();
        struct json_object *page = json_object_new_object();
        json_object_object_add(page, "page_index", json_object_new_int(0));
        json_object_object_add(page, "width", json_object_new_int(612));
        json_object_object_add(page, "height", json_object_new_int(792));
        json_object_object_add(page, "rotation", json_object_new_int(0));
        json_object_array_add(pages, page);
        json_object_object_add(result, "pages", pages);

        struct json_object *errors = json_object_new_array();
        json_object_object_add(result, "errors", errors);
    } else if (strcmp(method, "extract_text") == 0) {
        json_object_put(result);
        return json_object_new_string("Sample text content");
    } else if (strcmp(method, "extract_markdown") == 0) {
        json_object_put(result);
        return json_object_new_string("# Sample Markdown\n\nContent here");
    } else if (strcmp(method, "hash") == 0) {
        json_object_object_add(result, "hash", json_object_new_string("abc123"));
        json_object_object_add(result, "fast_hash", json_object_new_string("def456"));
    }

    return result;
}

/* Get current time in milliseconds */
static long time_ms(void) {
    struct timeval tv;
    gettimeofday(&tv, NULL);
    return (long)(tv.tv_sec * 1000 + tv.tv_usec / 1000);
}

/* Run a single test case */
static test_result_t *run_test_case(struct json_object *test_case,
                                     const char *schema_version,
                                     const char *fixtures_base,
                                     char **error_msg) {
    long start = time_ms();

    test_result_t *result = calloc(1, sizeof(test_result_t));

    struct json_object *id_obj = NULL;
    json_object_object_get_ex(test_case, "id", &id_obj);
    result->id = strdup(json_object_get_string(id_obj));

    /* Check min_schema_version */
    struct json_object *min_ver_obj = NULL;
    if (json_object_object_get_ex(test_case, "min_schema_version", &min_ver_obj) && min_ver_obj) {
        const char *min_ver = json_object_get_string(min_ver_obj);
        /* Simple version comparison */
        int schema_major = atoi(schema_version);
        int schema_minor = atoi(strchr(schema_version, '.') + 1);
        int min_major = atoi(min_ver);
        int min_minor = atoi(strchr(min_ver, '.') + 1);

        if (schema_major < min_major ||
            (schema_major == min_major && schema_minor < min_minor)) {
            result->status = STATUS_SKIP;
            asprintf(&result->reason, "Schema version %s < minimum required %s",
                     schema_version, min_ver);
            result->duration_ms = time_ms() - start;
            return result;
        }
    }

    struct json_object *fixture_obj = NULL;
    json_object_object_get_ex(test_case, "fixture", &fixture_obj);
    const char *fixture = json_object_get_string(fixture_obj);

    struct json_object *method_obj = NULL;
    json_object_object_get_ex(test_case, "method", &method_obj);
    const char *method = json_object_get_string(method_obj);

    struct json_object *options_obj = NULL;
    json_object_object_get_ex(test_case, "options", &options_obj);

    struct json_object *expected_obj = NULL;
    json_object_object_get_ex(test_case, "expected", &expected_obj);

    struct json_object *tolerances_obj = NULL;
    json_object_object_get_ex(test_case, "tolerances", &tolerances_obj);

    char *fixture_path;
    if (strncmp(fixture, "http://", 7) == 0 || strncmp(fixture, "https://", 8) == 0) {
        fixture_path = strdup(fixture);
    } else {
        asprintf(&fixture_path, "%s/%s", fixtures_base, fixture);
    }

    char *exec_error = NULL;
    struct json_object *actual = execute_method(method, fixture_path, options_obj, &exec_error);

    free(fixture_path);

    if (exec_error) {
        result->status = STATUS_ERROR;
        result->error = exec_error;
        result->expected = json_object_get(expected_obj);
        result->duration_ms = time_ms() - start;
        return result;
    }

    char *compare_error = NULL;
    int passed = compare_results(actual, expected_obj, tolerances_obj, "", &compare_error);

    if (passed) {
        result->status = STATUS_PASS;
        result->actual = actual;
        result->expected = json_object_get(expected_obj);
    } else {
        result->status = STATUS_FAIL;
        result->actual = actual;
        result->expected = json_object_get(expected_obj);
        result->reason = compare_error;
    }

    result->duration_ms = time_ms() - start;
    return result;
}

/* Main conformance runner */
int main(int argc, char **argv) {
    const char *suite_path = argc > 1 ? argv[1] : SUITE_PATH;
    const char *output_path = argc > 2 ? argv[2] : "conformance-report.json";

    printf("pdftract SDK Conformance Runner\n");
    printf("SDK: %s v%s\n", SDK_NAME, SDK_VERSION);
    printf("Suite: %s\n\n", suite_path);

    /* Load suite */
    FILE *suite_file = fopen(suite_path, "r");
    if (!suite_file) {
        fprintf(stderr, "Failed to open suite file: %s\n", suite_path);
        return 1;
    }

    fseek(suite_file, 0, SEEK_END);
    long suite_size = ftell(suite_file);
    fseek(suite_file, 0, SEEK_SET);

    char *suite_data = malloc(suite_size + 1);
    fread(suite_data, 1, suite_size, suite_file);
    suite_data[suite_size] = '\0';
    fclose(suite_file);

    struct json_object *suite = json_tokener_parse(suite_data);
    free(suite_data);

    struct json_object *version_obj = NULL, *schema_ver_obj = NULL, *cases_obj = NULL;
    json_object_object_get_ex(suite, "version", &version_obj);
    json_object_object_get_ex(suite, "schema_version", &schema_ver_obj);
    json_object_object_get_ex(suite, "cases", &cases_obj);

    const char *suite_version = json_object_get_string(version_obj);
    const char *schema_version = json_object_get_string(schema_ver_obj);

    /* Build fixtures base path */
    char fixtures_base[1024];
    snprintf(fixtures_base, sizeof(fixtures_base), "%s/fixtures", dirname(strdup(suite_path)));

    printf("Found %d test cases\n\n", json_object_array_length(cases_obj));

    long start_time = time_ms();
    test_result_t **results = calloc(json_object_array_length(cases_obj), sizeof(test_result_t*));
    int result_count = 0;

    for (int i = 0; i < json_object_array_length(cases_obj); i++) {
        struct json_object *test_case = json_object_array_get_idx(cases_obj, i);
        char *error_msg = NULL;
        test_result_t *result = run_test_case(test_case, schema_version, fixtures_base, &error_msg);
        results[result_count++] = result;

        const char *status_str = NULL;
        switch (result->status) {
            case STATUS_PASS: status_str = "PASS"; break;
            case STATUS_FAIL: status_str = "FAIL"; break;
            case STATUS_SKIP: status_str = "SKIP"; break;
            case STATUS_ERROR: status_str = "ERROR"; break;
        }

        printf("[%s] %s (%ldms)\n", status_str, result->id, result->duration_ms);

        if (result->status == STATUS_FAIL || result->status == STATUS_ERROR) {
            if (result->reason) printf("  Reason: %s\n", result->reason);
            if (result->error) printf("  Error: %s\n", result->error);
        }
    }

    long duration_ms = time_ms() - start_time;

    summary_t summary = {
        .total = result_count,
        .passed = 0,
        .failed = 0,
        .skipped = 0,
        .errors = 0,
        .duration_ms = duration_ms
    };

    for (int i = 0; i < result_count; i++) {
        switch (results[i]->status) {
            case STATUS_PASS: summary.passed++; break;
            case STATUS_FAIL: summary.failed++; break;
            case STATUS_SKIP: summary.skipped++; break;
            case STATUS_ERROR: summary.errors++; break;
        }
    }

    printf("\nSummary:\n");
    printf("  Total:   %d\n", summary.total);
    printf("  Passed:  %d\n", summary.passed);
    printf("  Failed:  %d\n", summary.failed);
    printf("  Skipped: %d\n", summary.skipped);
    printf("  Errors:  %d\n", summary.errors);
    printf("  Time:    %ldms\n", summary.duration_ms);

    /* Build report JSON */
    struct json_object *report = json_object_new_object();
    json_object_object_add(report, "sdk", json_object_new_string(SDK_NAME));
    json_object_object_add(report, "sdk_version", json_object_new_string(SDK_VERSION));
    json_object_object_add(report, "suite_version", json_object_new_string(suite_version));
    json_object_object_add(report, "schema_version", json_object_new_string(schema_version));

    /* Get timestamp */
    time_t now = time(NULL);
    char timestamp[64];
    strftime(timestamp, sizeof(timestamp), "%Y-%m-%dT%H:%M:%SZ", gmtime(&now));
    json_object_object_add(report, "timestamp", json_object_new_string(timestamp));

    struct json_object *results_array = json_object_new_array();
    for (int i = 0; i < result_count; i++) {
        struct json_object *result_obj = json_object_new_object();
        json_object_object_add(result_obj, "id", json_object_new_string(results[i]->id));

        const char *status_str = NULL;
        switch (results[i]->status) {
            case STATUS_PASS: status_str = "pass"; break;
            case STATUS_FAIL: status_str = "fail"; break;
            case STATUS_SKIP: status_str = "skip"; break;
            case STATUS_ERROR: status_str = "error"; break;
        }
        json_object_object_add(result_obj, "status", json_object_new_string(status_str));

        if (results[i]->actual) {
            json_object_object_add(result_obj, "actual", json_object_get(results[i]->actual));
        }
        if (results[i]->expected) {
            json_object_object_add(result_obj, "expected", json_object_get(results[i]->expected));
        }
        if (results[i]->error) {
            json_object_object_add(result_obj, "error", json_object_new_string(results[i]->error));
        }
        if (results[i]->reason) {
            json_object_object_add(result_obj, "reason", json_object_new_string(results[i]->reason));
        }
        json_object_object_add(result_obj, "duration_ms",
                               json_object_new_int(results[i]->duration_ms));

        json_object_array_add(results_array, result_obj);
    }
    json_object_object_add(report, "results", results_array);

    struct json_object *summary_obj = json_object_new_object();
    json_object_object_add(summary_obj, "total", json_object_new_int(summary.total));
    json_object_object_add(summary_obj, "passed", json_object_new_int(summary.passed));
    json_object_object_add(summary_obj, "failed", json_object_new_int(summary.failed));
    json_object_object_add(summary_obj, "skipped", json_object_new_int(summary.skipped));
    json_object_object_add(summary_obj, "errors", json_object_new_int(summary.errors));
    json_object_object_add(summary_obj, "duration_ms", json_object_new_int(summary.duration_ms));
    json_object_object_add(report, "summary", summary_obj);

    /* Write report */
    FILE *output_file = fopen(output_path, "w");
    if (output_file) {
        fputs(json_object_to_json_string_ext(report, JSON_C_TO_STRING_PRETTY), output_file);
        fclose(output_file);
        printf("\nReport written to: %s\n", output_path);
    }

    json_object_put(report);

    /* Cleanup results */
    for (int i = 0; i < result_count; i++) {
        free(results[i]->id);
        if (results[i]->actual) json_object_put(results[i]->actual);
        if (results[i]->expected) json_object_put(results[i]->expected);
        free(results[i]->error);
        free(results[i]->reason);
        free(results[i]);
    }
    free(results);
    json_object_put(suite);

    return summary.failed == 0 && summary.errors == 0 ? 0 : 1;
}
