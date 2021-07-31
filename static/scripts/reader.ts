// Variable to prevent fetching more than one set of messages at once
let fetching = false;

let $messages = $("#messages");

function scroll_to_bottom() {
    $messages = $("#messages");
    $messages.scrollTop($messages.prop("scrollHeight"));

    $("#messages .attachment").on("load", function() {
        $messages[0].scrollBy(0, $(this).prop("scrollHeight"));
    });
}

scroll_to_bottom();

declare function show_toast(message: string, duration?: number);

function jump(channel_id, message_id = undefined) {
    if (fetching) return;
    fetching = true;
    $.post("/jump", {
        chat_id: channel_id,
        message_id: message_id,
    }).done(result => {
        if (!result.name.length) {
            show_toast("channel not found");
            fetching = false;
            return;
        }
        // If channel_id is null, we're jumping to a message in the same channel
        if (channel_id) {
            // Update the cookie
            document.cookie = "chat=" + channel_id;
            // Select the new chat in the dropdown
            $("#chat").val(channel_id);
            // Update the chat name and topic
            $("#chat_header .name").text(result.name);
            $("#chat_header .topic").text(result.topic);
        }
        $messages.empty();
        // Display the new messages
        display_messages(result.messages, true);
        // Jump to the relevant message
        if (message_id) {
            document.querySelector(`[data-message-id="${message_id}"]`).scrollIntoView();
        } else {
            // Create and hide the bottom_loading div because we know we're caught up here
            $messages.append(`<div id="bottom_loading">Loading...</div>`);
            $("#bottom_loading").hide();
            // Scroll to the bottom of the messages
            scroll_to_bottom();
        }
        fetching = false;
    });
}

function init_handlers() {
    $(".content a").off().on("click", function(e) {
        // Check if it's a discord message link to jump
        let discord_link = this["href"].match(/^https?:\/\/(canary\.|ptb\.)?discord\.com\/channels\/\d+\/(\d+)\/(\d+)/i);
        // Check if it's a matrix link we need to ignore
        let matrix_link_ignore = this["href"].match(/https?:\/\/matrix\.to\/#\/@.*/i);
        // Check if it's a matrix message link to jump
        let matrix_link_message = this["href"].match(/https?:\/\/matrix\.to\/#\/(!.+:.+)\/(\$[^?]+)(\?.*)?/i);
        if (discord_link) {
            e.preventDefault();
            console.log(discord_link[2], discord_link[3])
            jump(discord_link[2], discord_link[3]);
        } else if (matrix_link_ignore) {
            e.preventDefault();
        } else if (matrix_link_message) {
            e.preventDefault();
            jump(matrix_link_message[1], matrix_link_message[2]);
        }
    });

    $(".spoiler").off().on("click", function() {
        $(this).addClass("opened");
    });

    $(".channel").off().on("click", function() {
        jump($(this).attr("data-id"));
    });

    $(".message.reply .parent").on("click", function() {
        jump(undefined, $(this).attr("data-id"));
    });
}

init_handlers();

function display_messages(messages, ascending) {
    for (let i = 0; i < messages.length; i++) {
        let message = messages[ascending ? i : messages.length - 1 - i];
        let html;
        // The `message` class is for JavaScript actions like counting the number of message, or
        // deleting messages. The `message_container` class is for styling with CSS, and in the
        // case of replies, there will be a separate div inside the `message` div having the
        // `message_container` class
        if (message.message_type === "day_separator") {
            // This can't be a reply so just add the message_container class
            html = `<div class="message message_container day_separator">
                <div class="line"></div>
                <div class="content">${message.content}</div>
                <div class="line"></div>
            </div>`;
        } else if (!message.separate) {
            // This can't be a reply so just add the message_container class
            html = `<div id="${message.sequential_id}" data-message-id="${message.message_id}" class="message message_container attached">
                <div class="spacer"></div>
                <div>
                    <div class="content" title="${message.created_timestamp}" data-bot="${message.bot}">${message.content}</div>
                </div>
            </div>`;
        } else if (message.message_type === "redacted") {
            html = `<div id="${message.sequential_id}" data-message-id="${message.message_id}" class="message redacted">
                <img src="${message.avatar}" alt="pfp" class="avatar">
                <div class="content">[redacted]</div>
            </div>`;
        } else if (message.message_type === "default") {
            // We need to check if this is a reply
            let parent = "";
            if (message.reference) {
                // SVG file icon to in place of attachments
                let attachment_icon = "<svg xmlns=\"http://www.w3.org/2000/svg\" height=\"24\" viewBox=\"0 0 24 24\" width=\"24\"><path d=\"M0 0h24v24H0z\" fill=\"none\"/><path d=\"M14 2H6c-1.1 0-1.99.9-1.99 2L4 20c0 1.1.89 2 1.99 2H18c1.1 0 2-.9 2-2V8l-6-6zm2 16H8v-2h8v2zm0-4H8v-2h8v2zm-3-5V3.5L18.5 9H13z\"/></svg>";
                // The parent message this is a reply to
                parent = `<div class="parent" data-id="${message.reference[0]}">
                    <img src="${message.reference[2]}" alt="pfp" class="avatar">
                    <span class="name" style="color: ${message.reference[3]}">${message.reference[1]}</span>
                    <span class="content">${message.reference[4]}</span>
                    ${message.reference[5] ? attachment_icon : ""}
                </div>
                <div class="message_container">`;
            }
            html = `<div id="${message.sequential_id}" data-message-id="${message.message_id}" class="message ${message.reference ? "reply" : "message_container"}">
                ${parent}<img src="${message.avatar}" alt="pfp" class="avatar">
                <div>
                    <div class="title">
                        <span class="name" style="color: ${message.color}">${message.name}</span>
                        <span class="timestamp">${message.created_timestamp}</span>
                        ${message.bot ? "<span class=\"bot\">BOT</span>" : ""}
                    </div>
                    <div class="content" title="${message.created_timestamp}">${message.content}</div>
                </div>${message.reference ? "</div>" : ""}
            </div>`;
        } else if (message.message_type === "new_member") {
            // This can't be a reply so just add the message_container class
            html = `<div id="${message.sequential_id}" data-message-id="${message.message_id}" class="message message_container">
                <div class="spacer">
                    <svg xmlns="http://www.w3.org/2000/svg" enable-background="new 0 0 24 24" height="24" viewBox="0 0 24 24" width="24"><rect fill="none" height="24" width="24"/><path d="M15,5l-1.41,1.41L18.17,11H2V13h16.17l-4.59,4.59L15,19l7-7L15,5z"/></svg>
                </div>
                <div>
                    <div class="content" title="${message.created_timestamp}" data-bot="${message.bot}">${message.content}</div>
                </div>
            </div>`;
        } else if (message.message_type === "pins_add") {
            // This can't be a reply so just add the message_container class
            html = `<div id="${message.sequential_id}" data-message-id="${message.message_id}" class="message message_container">
                <div class="spacer">
                    <svg xmlns="http://www.w3.org/2000/svg" enable-background="new 0 0 24 24" height="24" viewBox="0 0 24 24" width="24"><g><rect fill="none" height="24" width="24"/></g><g><path d="M16,9V4l1,0c0.55,0,1-0.45,1-1v0c0-0.55-0.45-1-1-1H7C6.45,2,6,2.45,6,3v0 c0,0.55,0.45,1,1,1l1,0v5c0,1.66-1.34,3-3,3h0v2h5.97v7l1,1l1-1v-7H19v-2h0C17.34,12,16,10.66,16,9z" fill-rule="evenodd"/></g></svg>
                </div>
                <div>
                    <div class="content" title="${message.created_timestamp}" data-bot="${message.bot}">${message.content}</div>
                </div>
            </div>`;
        } else {
            // This can't be a reply so just add the message_container class
            html = `<div id="${message.sequential_id}" data-message-id="${message.message_id}" class="message message_container">
                <div class="spacer"></div>
                <div>
                    <div class="content" title="${message.created_timestamp}" data-bot="${message.bot}">${message.content}</div>
                </div>
            </div>`;
        }

        if (ascending) {
            // If the messages are in chronological order
            $messages.append(html);
        } else {
            // If the messages are in reverse chronological order
            $messages.prepend(html);
        }

        // Add the edited sign
        if (message.edited_timestamp) {
            $(`#messages .message:${ascending ? "last" : "first"}-child div.content`)
                .append(`<div class="timestamp" title="edited at ${message.edited_timestamp}">(edited)</div>`);
        }

        // Add any attachments to the message
        for (let j = 0; j < message.attachments.length; j++) {
            let attachment = message.attachments[j];
            let html = "";
            if (attachment[2]) html += "<div class=\"spoiler\">";
            if (attachment[1] === "image") {
                html += `<img src="${attachment[0]}" alt="attachment" class="attachment">`;
            } else if (attachment[1] === "video") {
                html += `<video src="${attachment[0]}" class="attachment" controls></video>`;
            } else if (attachment[1] === "audio") {
                html += `<audio src="${attachment[0]}" class="attachment" controls></audio>`;
            } else {
                html += `<div class="generic_attachment">
                    <a href="${attachment[0]}">${attachment[0].split("/")[attachment[0].split("/").length - 1]}</a>
                </div>`;
            }
            if (attachment[2]) html += "</div>";
            // If they're in chronological order, the message is at the bottom
            // We're doing `div.content` because if it's a reply the parent will have `span.content`
            $(`#messages .message:${ascending ? "last" : "first"}-child div.content`).after(html);
        }

        // Add any reactions to the message
        if (message.reactions.length) {
            let html = "<div class=\"reactions\">";
            for (let j = 0; j < message.reactions.length; j++) {
                let reaction = message.reactions[j];
                // The text we're appending ends with a space so we have some spacing between reactions
                if (reaction[1]) {
                    // It's a custom emoji
                    html += `<div class="reaction">
                    <img src="${reaction[1]}" alt="${reaction[0]}" title="${reaction[0]}" class="emoji">
                    ${reaction[2]}
                </div> `;
                } else {
                    // It's a unicode emoji
                    html += `<div class="reaction">${reaction[0]} ${reaction[2]}</div> `;
                }
            }
            html += "</div>";
            // If they're in chronological order, the message is at the bottom
            $(`#messages .message:${ascending ? "last" : "first"}-child > div:last-child`).append(html);
        }
    }
    init_handlers();
}

$messages.on("scroll", function() {
    // If the top_loading div exists, we shouldn't initiate a new POST request
    // We can't rely solely on `fetching` because we might be at the beginning of messages
    if ($(this).scrollTop() === 0 && $("#top_loading").length === 0 && !fetching) {
        fetching = true;
        // Add the top_loading div
        $messages.prepend(`<div id="top_loading">Loading...</div>`);
        // Store the ID so we can scroll back to it after adding more messages
        const reference_message = $("#messages .message")[0].id;
        // Fetch the messages
        $.post("/messages", {sequential_id: reference_message, position: "above"}).done(result => {
            const $top_loading = $("#top_loading");
            if (result.length === 0) {
                fetching = false;
                // If there are no more messages, we hide the top_loading div instead of removing it
                // This is so we don't attempt fetching messages when the user scrolls here again
                $top_loading.hide();
                return;
            }

            display_messages(result, false);
            // Delete other messages if there are too many in the DOM
            while ($("#messages .message:not(.day_separator)").length > 300) {
                // Remove the bottom_loading div if it exists so that the messages will be fetched
                // Also necessary for :last-child to work
                $("#bottom_loading").remove();
                $("#messages .message:last-child").remove();
            }

            // Scroll back to the message the user was at
            document.getElementById(reference_message).scrollIntoView();
            $top_loading.remove();
            fetching = false;
        });
    } else if ($messages.scrollTop() + $messages.prop("offsetHeight") >= $messages.prop("scrollHeight") && $("#bottom_loading").length === 0 && !fetching) {
        fetching = true;
        // Add the bottom_loading div
        $messages.append(`<div id="bottom_loading">Loading...</div>`);
        $.post("/messages", {sequential_id: $("#messages .message:last")[0].id, position: "below"}).done(result => {
            const $bottom_loading = $("#bottom_loading");
            if (result.length === 0) {
                fetching = false;
                // If there are no more messages, we hide the bottom_loading div instead of removing it
                // This is so we don't attempt fetching messages when the user scrolls here again
                $bottom_loading.hide();
                return;
            }

            display_messages(result, true);
            // Delete other messages if there are too many in the DOM
            while ($("#messages .message:not(.day_separator)").length > 300) {
                // Remove the top_loading div if it exists so that the messages will be fetched
                // Also necessary for :first-child to work
                $("#top_loading").remove();
                $("#messages .message:first-child").remove();
            }

            // No need to scroll back to the message the user was at since the elements are added to the bottom
            $bottom_loading.remove();
            fetching = false;
        });
    }
});

// Keyboard shortcuts
$(window).on("keydown", e => {
    // Ctrl + F to focus the search box
    if (e.ctrlKey && e.key === "f") {
        e.preventDefault();
        $("#query").trigger("focus");
    }
});

$("#search form").on("submit", e => {
    e.preventDefault();
    const $search_button = $("#search form button");
    const $results = $("#results");
    const query = $("#query").val() as string;
    $search_button.prop("disabled", true);

    if (query.length === 0) {
        // If both the query is empty, remove any existing results
        $results.empty();
        $search_button.prop("disabled", false);
        return;
    }
    $.post("/search", {string: query, filters: $("#filters").val()}).done(result => {
        // Clear any existing search results
        $results.empty();
        // Show the search results
        for (let i = 0; i < result.length; i++) {
            let message = result[i];
            if (message.message_type !== "default") {
                // Don't render day separators
                continue;
            }
            $results.prepend(`
                <div class="message message_container" data-id="${message.sequential_id}">
                    <img src="${message.avatar}" alt="pfp" class="avatar">
                    <div>
                        <div class="title">
                            <span class="name" style="color: ${message.color}">${message.name}</span>
                            <span class="timestamp">${message.created_timestamp}</span>
                        </div>
                        <div class="content" data-bot="${message.bot}">${message.content}</div>
                    </div>
                </div>
            `);
            // Add any attachments to the message
            for (let j = 0; j < message.attachments.length; j++) {
                let attachment = message.attachments[j];
                if (attachment[1] === "image") {
                    $(`#results .message[data-id="${message.sequential_id}"] > div`).append(`<img src="${attachment[0]}" alt="attachment" class="attachment">`);
                } else if (attachment[1] === "video") {
                    $(`#results .message[data-id="${message.sequential_id}"] > div`).append(`<video src="${attachment[0]}" class="attachment" controls></video>`);
                } else if (attachment[1] === "audio") {
                    $(`#results .message[data-id="${message.sequential_id}"] > div`).append(`<audio src="${attachment[0]}" class="attachment" controls></audio>`);
                }
            }
        }
        $results.prepend(`<div id="info">${result.length} result${result.length === 1 ? "" : "s"}</div>`);
        $search_button.prop("disabled", false);
        $("#results").scrollTop(0);
        $("#results .message").on("click", function() {
            // To keep track of what message to scroll to when a search result is clicked
            let clicked = $(this).attr("data-id");
            // Fetch the messages around the clicked message
            $.post("/messages", {sequential_id: $(this).attr("data-id"), position: "around"}).done(result => {
                $messages.empty();
                display_messages(result, true);
                // Jump to the clicked message
                document.getElementById(clicked).scrollIntoView();
                $("#top_loading, #bottom_loading").remove();
            });
        });
    }).fail(error => {
        $search_button.prop("disabled", false);
        console.log(error);
        $("#results").html(`<div id="info">${error.statusText}</div>`);
    });
});
