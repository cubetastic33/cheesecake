$.getJSON("https://api.github.com/repos/cubetastic33/cheesecake/releases/latest", function (result) {
    if (result["tag_name"] !== version) {
        $("#outdated").show();
    }
});
var $backup = $("#backup");
function show_toast(message, duration) {
    if (duration === void 0) { duration = 2000; }
    $("#toast").text(message).slideDown(200, function () {
        setTimeout(function () {
            $("#toast").slideUp(200);
        }, duration);
    });
}
function display_chats(backup) {
    $("#chat").prop("outerHTML", "<select id=\"chat\"></select>");
    $("label[for=\"chat\"]").text("Chat:");
    for (var i = 0; i < chats[backup].length; i++) {
        var chat = chats[backup][i];
        $("#chat").append("<option value=\"" + chat[0] + "\">" + chat[1] + "</option>");
    }
}
$("#chat_switcher").trigger("reset").on("submit", function (e) {
    e.preventDefault();
    var $chat = $("#chat");
    if ($chat.is("input")) {
        if ($chat.val()) {
            document.cookie = "backup=" + $("#backup").val();
            $("#chat_switcher button")
                .prop("disabled", true)
                .text("decrypting...");
            $backup.prop("disabled", true);
            $.post("/decrypt", { "password": $chat.val() }).done(function (result) {
                console.log(result);
                if (result.length) {
                    chats = initial_chats;
                    var backup = $backup.val();
                    chats[backup] = result;
                    display_chats(backup);
                    show_toast("The backup will remain decrypted until you open a different backup or quit cheesecake", 5000);
                }
                else {
                    show_toast("No chats found. Is your password incorrect?");
                }
                $backup.prop("disabled", false);
                $("#chat_switcher button")
                    .text("proceed")
                    .prop("disabled", false);
            });
        }
    }
    else {
        document.cookie = "backup=" + $("#backup").val();
        document.cookie = "chat=" + $chat.val();
        location.href = "/reader";
    }
});
$backup.on("change", function () {
    var backup = $backup.val();
    if (!Object.keys(chats[backup]).length) {
        $("#chat").prop("outerHTML", "<input type=\"password\" id=\"chat\" placeholder=\"password\">");
        $("label[for=\"chat\"]").text("Password:");
    }
    else {
        display_chats(backup);
    }
});
//# sourceMappingURL=main.js.map