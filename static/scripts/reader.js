var fetching = false;
var $messages = $("#messages");
function scroll_to_bottom() {
    $messages = $("#messages");
    $messages.scrollTop($messages.prop("scrollHeight"));
    $("#messages .attachment").on("load", function () {
        $messages[0].scrollBy(0, $(this).prop("scrollHeight"));
    });
}
scroll_to_bottom();
function jump(channel_id, message_id) {
    if (message_id === void 0) { message_id = undefined; }
    if (fetching)
        return;
    fetching = true;
    $.post("/jump", {
        chat_id: channel_id,
        message_id: message_id,
    }).done(function (result) {
        if (!result.name.length) {
            show_toast("channel not found");
            fetching = false;
            return;
        }
        if (channel_id) {
            document.cookie = "chat=" + channel_id;
            $("#chat").val(channel_id);
            $("#chat_header .name").text(result.name);
            $("#chat_header .topic").text(result.topic);
        }
        $messages.empty();
        display_messages(result.messages, true);
        if (message_id) {
            document.querySelector("[data-message-id=\"" + message_id + "\"]").scrollIntoView();
        }
        else {
            $messages.append("<div id=\"bottom_loading\">Loading...</div>");
            $("#bottom_loading").hide();
            scroll_to_bottom();
        }
        fetching = false;
    });
}
function init_handlers() {
    $(".content a").off().on("click", function (e) {
        var discord_link = this["href"].match(/^https?:\/\/(canary\.|ptb\.)?discord\.com\/channels\/\d+\/(\d+)\/(\d+)/i);
        var matrix_link_ignore = this["href"].match(/https?:\/\/matrix\.to\/#\/@.*/i);
        var matrix_link_message = this["href"].match(/https?:\/\/matrix\.to\/#\/(!.+:.+)\/(\$[^?]+)(\?.*)?/i);
        if (discord_link) {
            e.preventDefault();
            console.log(discord_link[2], discord_link[3]);
            jump(discord_link[2], discord_link[3]);
        }
        else if (matrix_link_ignore) {
            e.preventDefault();
        }
        else if (matrix_link_message) {
            e.preventDefault();
            jump(matrix_link_message[1], matrix_link_message[2]);
        }
    });
    $(".spoiler").off().on("click", function () {
        $(this).addClass("opened");
    });
    $(".channel").off().on("click", function () {
        jump($(this).attr("data-id"));
    });
    $(".message.reply .parent").on("click", function () {
        jump(undefined, $(this).attr("data-id"));
    });
}
init_handlers();
function display_messages(messages, ascending) {
    for (var i = 0; i < messages.length; i++) {
        var message = messages[ascending ? i : messages.length - 1 - i];
        var html = void 0;
        if (message.message_type === "day_separator") {
            html = "<div class=\"message message_container day_separator\">\n                <div class=\"line\"></div>\n                <div class=\"content\">" + message.content + "</div>\n                <div class=\"line\"></div>\n            </div>";
        }
        else if (!message.separate) {
            html = "<div id=\"" + message.sequential_id + "\" data-message-id=\"" + message.message_id + "\" class=\"message message_container attached\">\n                <div class=\"spacer\"></div>\n                <div>\n                    <div class=\"content\" title=\"" + message.created_timestamp + "\" data-bot=\"" + message.bot + "\">" + message.content + "</div>\n                </div>\n            </div>";
        }
        else if (message.message_type === "redacted") {
            html = "<div id=\"" + message.sequential_id + "\" data-message-id=\"" + message.message_id + "\" class=\"message redacted\">\n                <img src=\"" + message.avatar + "\" alt=\"pfp\" class=\"avatar\">\n                <div class=\"content\">[redacted]</div>\n            </div>";
        }
        else if (message.message_type === "default") {
            var parent_1 = "";
            if (message.reference) {
                var attachment_icon = "<svg xmlns=\"http://www.w3.org/2000/svg\" height=\"24\" viewBox=\"0 0 24 24\" width=\"24\"><path d=\"M0 0h24v24H0z\" fill=\"none\"/><path d=\"M14 2H6c-1.1 0-1.99.9-1.99 2L4 20c0 1.1.89 2 1.99 2H18c1.1 0 2-.9 2-2V8l-6-6zm2 16H8v-2h8v2zm0-4H8v-2h8v2zm-3-5V3.5L18.5 9H13z\"/></svg>";
                parent_1 = "<div class=\"parent\" data-id=\"" + message.reference[0] + "\">\n                    <img src=\"" + message.reference[2] + "\" alt=\"pfp\" class=\"avatar\">\n                    <span class=\"name\" style=\"color: " + message.reference[3] + "\">" + message.reference[1] + "</span>\n                    <span class=\"content\">" + message.reference[4] + "</span>\n                    " + (message.reference[5] ? attachment_icon : "") + "\n                </div>\n                <div class=\"message_container\">";
            }
            html = "<div id=\"" + message.sequential_id + "\" data-message-id=\"" + message.message_id + "\" class=\"message " + (message.reference ? "reply" : "message_container") + "\">\n                " + parent_1 + "<img src=\"" + message.avatar + "\" alt=\"pfp\" class=\"avatar\">\n                <div>\n                    <div class=\"title\">\n                        <span class=\"name\" style=\"color: " + message.color + "\">" + message.name + "</span>\n                        <span class=\"timestamp\">" + message.created_timestamp + "</span>\n                        " + (message.bot ? "<span class=\"bot\">BOT</span>" : "") + "\n                    </div>\n                    <div class=\"content\" title=\"" + message.created_timestamp + "\">" + message.content + "</div>\n                </div>" + (message.reference ? "</div>" : "") + "\n            </div>";
        }
        else if (message.message_type === "new_member") {
            html = "<div id=\"" + message.sequential_id + "\" data-message-id=\"" + message.message_id + "\" class=\"message message_container\">\n                <div class=\"spacer\">\n                    <svg xmlns=\"http://www.w3.org/2000/svg\" enable-background=\"new 0 0 24 24\" height=\"24\" viewBox=\"0 0 24 24\" width=\"24\"><rect fill=\"none\" height=\"24\" width=\"24\"/><path d=\"M15,5l-1.41,1.41L18.17,11H2V13h16.17l-4.59,4.59L15,19l7-7L15,5z\"/></svg>\n                </div>\n                <div>\n                    <div class=\"content\" title=\"" + message.created_timestamp + "\" data-bot=\"" + message.bot + "\">" + message.content + "</div>\n                </div>\n            </div>";
        }
        else if (message.message_type === "pins_add") {
            html = "<div id=\"" + message.sequential_id + "\" data-message-id=\"" + message.message_id + "\" class=\"message message_container\">\n                <div class=\"spacer\">\n                    <svg xmlns=\"http://www.w3.org/2000/svg\" enable-background=\"new 0 0 24 24\" height=\"24\" viewBox=\"0 0 24 24\" width=\"24\"><g><rect fill=\"none\" height=\"24\" width=\"24\"/></g><g><path d=\"M16,9V4l1,0c0.55,0,1-0.45,1-1v0c0-0.55-0.45-1-1-1H7C6.45,2,6,2.45,6,3v0 c0,0.55,0.45,1,1,1l1,0v5c0,1.66-1.34,3-3,3h0v2h5.97v7l1,1l1-1v-7H19v-2h0C17.34,12,16,10.66,16,9z\" fill-rule=\"evenodd\"/></g></svg>\n                </div>\n                <div>\n                    <div class=\"content\" title=\"" + message.created_timestamp + "\" data-bot=\"" + message.bot + "\">" + message.content + "</div>\n                </div>\n            </div>";
        }
        else {
            html = "<div id=\"" + message.sequential_id + "\" data-message-id=\"" + message.message_id + "\" class=\"message message_container\">\n                <div class=\"spacer\"></div>\n                <div>\n                    <div class=\"content\" title=\"" + message.created_timestamp + "\" data-bot=\"" + message.bot + "\">" + message.content + "</div>\n                </div>\n            </div>";
        }
        if (ascending) {
            $messages.append(html);
        }
        else {
            $messages.prepend(html);
        }
        if (message.edited_timestamp) {
            $("#messages .message:" + (ascending ? "last" : "first") + "-child div.content")
                .append("<div class=\"timestamp\" title=\"edited at " + message.edited_timestamp + "\">(edited)</div>");
        }
        for (var j = 0; j < message.attachments.length; j++) {
            var attachment = message.attachments[j];
            var html_1 = "";
            if (attachment[2])
                html_1 += "<div class=\"spoiler\">";
            if (attachment[1] === "image") {
                html_1 += "<img src=\"" + attachment[0] + "\" alt=\"attachment\" class=\"attachment\">";
            }
            else if (attachment[1] === "video") {
                html_1 += "<video src=\"" + attachment[0] + "\" class=\"attachment\" controls></video>";
            }
            else if (attachment[1] === "audio") {
                html_1 += "<audio src=\"" + attachment[0] + "\" class=\"attachment\" controls></audio>";
            }
            else {
                html_1 += "<div class=\"generic_attachment\">\n                    <a href=\"" + attachment[0] + "\">" + attachment[0].split("/")[attachment[0].split("/").length - 1] + "</a>\n                </div>";
            }
            if (attachment[2])
                html_1 += "</div>";
            $("#messages .message:" + (ascending ? "last" : "first") + "-child div.content").after(html_1);
        }
        if (message.reactions.length) {
            var html_2 = "<div class=\"reactions\">";
            for (var j = 0; j < message.reactions.length; j++) {
                var reaction = message.reactions[j];
                if (reaction[1]) {
                    html_2 += "<div class=\"reaction\">\n                    <img src=\"" + reaction[1] + "\" alt=\"" + reaction[0] + "\" title=\"" + reaction[0] + "\" class=\"emoji\">\n                    " + reaction[2] + "\n                </div> ";
                }
                else {
                    html_2 += "<div class=\"reaction\">" + reaction[0] + " " + reaction[2] + "</div> ";
                }
            }
            html_2 += "</div>";
            $("#messages .message:" + (ascending ? "last" : "first") + "-child > div:last-child").append(html_2);
        }
    }
    init_handlers();
}
$messages.on("scroll", function () {
    if ($(this).scrollTop() === 0 && $("#top_loading").length === 0 && !fetching) {
        fetching = true;
        $messages.prepend("<div id=\"top_loading\">Loading...</div>");
        var reference_message_1 = $("#messages .message")[0].id;
        $.post("/messages", { sequential_id: reference_message_1, position: "above" }).done(function (result) {
            var $top_loading = $("#top_loading");
            if (result.length === 0) {
                fetching = false;
                $top_loading.hide();
                return;
            }
            display_messages(result, false);
            while ($("#messages .message:not(.day_separator)").length > 300) {
                $("#bottom_loading").remove();
                $("#messages .message:last-child").remove();
            }
            document.getElementById(reference_message_1).scrollIntoView();
            $top_loading.remove();
            fetching = false;
        });
    }
    else if ($messages.scrollTop() + $messages.prop("offsetHeight") >= $messages.prop("scrollHeight") && $("#bottom_loading").length === 0 && !fetching) {
        fetching = true;
        $messages.append("<div id=\"bottom_loading\">Loading...</div>");
        $.post("/messages", { sequential_id: $("#messages .message:last")[0].id, position: "below" }).done(function (result) {
            var $bottom_loading = $("#bottom_loading");
            if (result.length === 0) {
                fetching = false;
                $bottom_loading.hide();
                return;
            }
            display_messages(result, true);
            while ($("#messages .message:not(.day_separator)").length > 300) {
                $("#top_loading").remove();
                $("#messages .message:first-child").remove();
            }
            $bottom_loading.remove();
            fetching = false;
        });
    }
});
$(window).on("keydown", function (e) {
    if (e.ctrlKey && e.key === "f") {
        e.preventDefault();
        $("#query").trigger("focus");
    }
});
$("#search form").on("submit", function (e) {
    e.preventDefault();
    var $search_button = $("#search form button");
    var $results = $("#results");
    var query = $("#query").val();
    $search_button.prop("disabled", true);
    if (query.length === 0) {
        $results.empty();
        $search_button.prop("disabled", false);
        return;
    }
    $.post("/search", { string: query, filters: $("#filters").val() }).done(function (result) {
        $results.empty();
        for (var i = 0; i < result.length; i++) {
            var message = result[i];
            if (message.message_type !== "default") {
                continue;
            }
            $results.prepend("\n                <div class=\"message message_container\" data-id=\"" + message.sequential_id + "\">\n                    <img src=\"" + message.avatar + "\" alt=\"pfp\" class=\"avatar\">\n                    <div>\n                        <div class=\"title\">\n                            <span class=\"name\" style=\"color: " + message.color + "\">" + message.name + "</span>\n                            <span class=\"timestamp\">" + message.created_timestamp + "</span>\n                        </div>\n                        <div class=\"content\" data-bot=\"" + message.bot + "\">" + message.content + "</div>\n                    </div>\n                </div>\n            ");
            for (var j = 0; j < message.attachments.length; j++) {
                var attachment = message.attachments[j];
                if (attachment[1] === "image") {
                    $("#results .message[data-id=\"" + message.sequential_id + "\"] > div").append("<img src=\"" + attachment[0] + "\" alt=\"attachment\" class=\"attachment\">");
                }
                else if (attachment[1] === "video") {
                    $("#results .message[data-id=\"" + message.sequential_id + "\"] > div").append("<video src=\"" + attachment[0] + "\" class=\"attachment\" controls></video>");
                }
                else if (attachment[1] === "audio") {
                    $("#results .message[data-id=\"" + message.sequential_id + "\"] > div").append("<audio src=\"" + attachment[0] + "\" class=\"attachment\" controls></audio>");
                }
            }
        }
        $results.prepend("<div id=\"info\">" + result.length + " result" + (result.length === 1 ? "" : "s") + "</div>");
        $search_button.prop("disabled", false);
        $("#results").scrollTop(0);
        $("#results .message").on("click", function () {
            var clicked = $(this).attr("data-id");
            $.post("/messages", { sequential_id: $(this).attr("data-id"), position: "around" }).done(function (result) {
                $messages.empty();
                display_messages(result, true);
                document.getElementById(clicked).scrollIntoView();
                $("#top_loading, #bottom_loading").remove();
            });
        });
    }).fail(function (error) {
        $search_button.prop("disabled", false);
        console.log(error);
        $("#results").html("<div id=\"info\">" + error.statusText + "</div>");
    });
});
//# sourceMappingURL=reader.js.map