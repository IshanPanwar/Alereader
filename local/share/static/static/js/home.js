$(function() {
  $('ul.left-menu li > ul').hide();
  $('ul.left-menu li').click(function(e){
    e.stopPropagation();
    $(this).children('ul').slideToggle();
  });
  $('a.feed-link').click(function(e){
    e.preventDefault();
    $('.viewpane').html('<div class="fa-2x d-flex justify-content-center align-items-center"><i class="fa-solid fa-volleyball fa-bounce" style="color:#7A306C"></i></div>');
    var title = $(this).attr('title');
    var parts = title.split('-');
    if (parts.length >=2){
      $.get('/'+parts[0]+'/'+parts[1]+'/', function(data) {
        $('.viewpane').html(data);
        $('.viewpane-view').find('img').addClass('img-fluid');
      }).fail(function() {
          console.log("Failed to fetch content from path");
        });
    }
  });
  $('a.feed-link').bind("contextmenu", function(e){
    e.preventDefault();
    $('.viewpane').html('<div class="fa-2x d-flex justify-content-center align-items-center"><i class="fa-solid fa-volleyball fa-bounce" style="color:#7A306C"></i></div>');
    var title = $(this).attr('title');
    var parts = title.split('-');
    if (parts.length >=2){
      $.get('/'+'force'+'/'+parts[0]+'/'+parts[1]+'/', function(data) {
        $('.viewpane').html(data);
        $('.viewpane-view').find('img').addClass('img-fluid');
      }).fail(function() {
          console.log("Failed to fetch content from path");
        });
    }
  });

  $('a.class-link').bind("contextmenu", function(e){
    e.preventDefault();
    $('.viewpane').html('<div class="fa-2x d-flex justify-content-center align-items-center"><i class="fa-solid fa-volleyball fa-bounce" style="color:#7A306C"></i></div>');
    var title = $(this).attr('title');
    console.log('/'+title+'/')
    $.get('/'+title+'/', function(data) {
      $('.viewpane').html(data);
      $('.viewpane-view').find('img').addClass('img-fluid');
    }).fail(function() {
        console.log("Failed to fetch content from path");
      });
  });
});
